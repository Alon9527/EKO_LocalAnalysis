import csv
from datetime import date
import io
import json
from pathlib import Path
import tempfile
import threading
import unittest
import urllib.error
import urllib.request
from urllib.parse import urlencode
import re

from eko_prospect.store import Store
from eko_prospect.policy import score
from eko_prospect.export import export_csv, portfolio
from eko_prospect.outreach import prepare_draft, record_sent
from eko_prospect.web import make_server


AS_OF = date(2026,11,1)


def qualified(db, domain='example.com', email='vendor@example.com'):
    cid=db.upsert_company('=ACME',f'https://{domain}','US')
    s=db.add_source(cid,url=f'https://{domain}/about',text='United States. Trash cans. Wholesale distributor.',retrieved_at='2026-11-01T00:00:00Z')
    s2=db.add_source(cid,url=f'https://{domain}/suppliers',text=f'Supplier application. Premium stainless steel products. {email}',retrieved_at='2026-11-01T00:00:00Z')
    for sid,kind,quote,value in [(s,'market','United States.','US'),(s,'category','Trash cans.',''),(s,'channel','Wholesale distributor.',''),(s2,'supplier_access',f'Supplier application. Premium stainless steel products. {email}',email),(s2,'commercial_fit','Premium stainless steel products.',''),(s2,'email',email,email)]:
        claim=db.add_claim(cid,sid,kind,quote,value=value)
        db.review_claim(claim,'verified','Tester','Synthetic fixture; manual approval simulated for tests only')
    db.review_company(cid,'approved','Tester','CRM checked',relationship='new',angle='建议评估商用桶适配，非采购事实',as_of=AS_OF,expected_revision=db.company(cid)['revision'])
    return cid


class WorkflowTests(unittest.TestCase):
    def setUp(self):
        self.tmp=tempfile.TemporaryDirectory()
        self.path=Path(self.tmp.name)/'db.sqlite'
        self.db=Store(self.path)

    def tearDown(self):
        self.db.close()
        self.tmp.cleanup()

    def test_end_to_end_review_and_csv_has_provenance(self):
        cid=qualified(self.db)
        self.assertTrue(score(self.db,cid,AS_OF)['qualified'])
        path=Path(self.tmp.name)/'export.csv'
        export_csv(self.db,path,AS_OF,'US')
        rows=list(csv.DictReader(io.StringIO(path.read_text(encoding='utf-8-sig'))))
        self.assertEqual(len(rows),1)
        self.assertTrue(rows[0]['name'].startswith("'="))
        self.assertEqual(rows[0]['score'],'8')
        self.assertEqual(rows[0]['verified_procurement'],'False')
        self.assertIn('https://example.com/about',rows[0]['evidence_json'])
        self.assertEqual(portfolio(self.db,AS_OF,'US')['qualified'],1)

    def test_draft_needs_exact_confirmation_and_blocks_duplicate(self):
        cid=qualified(self.db)
        with self.assertRaises(ValueError):
            prepare_draft(self.db,cid,'vendor@example.com','Hello','Text','Tester','wrong',AS_OF)
        draft=prepare_draft(self.db,cid,'vendor@example.com','Hello','Text','Tester','vendor@example.com',AS_OF)
        self.assertIn('To: vendor@example.com',draft['eml'])
        with self.assertRaises(ValueError):
            prepare_draft(self.db,cid,'vendor@example.com','Hello','Text','Tester','vendor@example.com',AS_OF)
        self.assertEqual(self.db.rows('SELECT status FROM outreach')[0]['status'],'prepared')
        record_sent(self.db,draft['id'],'Tester','Sent manually from business mailbox')
        self.assertEqual(self.db.rows('SELECT status FROM outreach')[0]['status'],'sent')

    def test_cross_company_same_email_and_group_blocked(self):
        a=qualified(self.db)
        b=qualified(self.db,'other.com','vendor@example.com')
        prepare_draft(self.db,a,'vendor@example.com','Hello','Text','Tester','vendor@example.com',AS_OF)
        with self.assertRaises(ValueError):
            prepare_draft(self.db,b,'vendor@example.com','Hello','Text','Tester','vendor@example.com',AS_OF)

    def test_changed_evidence_and_suppressed_email_block_drafts(self):
        cid=qualified(self.db)
        self.db.suppress('vendor@example.com','Do not contact','Tester')
        with self.assertRaises(ValueError):
            prepare_draft(self.db,cid,'vendor@example.com','Hello','Text','Tester','vendor@example.com',AS_OF)
        first=self.db.evidence(cid)[0]['id']
        self.db.review_claim(first,'rejected','Tester','Withdrawn')
        self.assertFalse(score(self.db,cid,AS_OF)['qualified'])

    def test_group_count_dedup(self):
        a=qualified(self.db)
        b=qualified(self.db,'other.com','vendor@other.com')
        self.db.review_company(b,'approved','Tester','Same parent',relationship='new',group_key='example.com',angle='建议商用桶',as_of=AS_OF,expected_revision=self.db.company(b)['revision'])
        self.assertEqual(portfolio(self.db,AS_OF,'US')['qualified'],1)

    def test_web_csrf_and_escaping(self):
        qualified(self.db)
        server=make_server(str(self.path),port=0,as_of=AS_OF)
        thread=threading.Thread(target=server.serve_forever,daemon=True)
        thread.start()
        base=f'http://127.0.0.1:{server.server_port}'
        try:
            with urllib.request.urlopen(base) as response:
                html=response.read().decode()
            self.assertIn('EKO',html)
            with urllib.request.urlopen(base+'/company?id=1') as response:
                detail=response.read().decode()
            self.assertIn('name="expected_revision"',detail)
            token=re.search('name="csrf" value="([^"]+)"',detail).group(1)
            data=urlencode({'csrf':token,'id':'1','status':'pending','actor':'UI-test','note':'Temporary test review'}).encode()
            req=urllib.request.Request(base+'/review-claim',data=data,headers={'Origin':base},method='POST')
            with urllib.request.urlopen(req) as response:
                self.assertIn('pending',response.read().decode())
            self.assertEqual(self.db.claim(1)['status'],'pending')
            malicious='<script>alert(1)</script>'
            sid=self.db.add_source(1,url='https://example.com/xss',text=malicious+' Trash cans.',retrieved_at='2026-11-01T00:00:00Z')
            self.db.add_claim(1,sid,'category',malicious+' Trash cans.')
            with urllib.request.urlopen(base+'/company?id=1') as response:
                escaped=response.read().decode()
            self.assertNotIn(malicious,escaped)
            self.assertIn('&lt;script&gt;alert(1)&lt;/script&gt;',escaped)
            req=urllib.request.Request(base+'/review-claim',data=b'id=1&status=verified',method='POST')
            with self.assertRaises(urllib.error.HTTPError) as error:
                urllib.request.urlopen(req)
            self.assertEqual(error.exception.code,403)
            req=urllib.request.Request(base+'/',headers={'Host':'evil.example'})
            with self.assertRaises(urllib.error.HTTPError) as error:
                urllib.request.urlopen(req)
            self.assertEqual(error.exception.code,403)
        finally:
            server.shutdown(); server.server_close(); thread.join()


if __name__=='__main__':
    unittest.main()
