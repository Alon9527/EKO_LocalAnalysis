"""Independent-review regressions. Inputs are synthetic, not customer facts."""
from datetime import date
from pathlib import Path
import os
import subprocess
import sys
import tempfile
import unittest
from unittest.mock import patch
import urllib.request
from eko_prospect.store import Store
from eko_prospect.policy import score
from eko_prospect.fetch import Fetcher,Response,TrustedProxyTransport
from eko_prospect.pipeline import collect
from eko_prospect.extract import RuleExtractor
from eko_prospect.outreach import prepare_draft
from test_acquisition import FakeTransport,html
from test_workflow import qualified,AS_OF


class ReviewRegressions(unittest.TestCase):
    def setUp(self):
        self.db=Store(':memory:')

    def tearDown(self):
        self.db.close()

    def test_first_party_subdomains_are_not_independent(self):
        cid=self.db.upsert_company('Example','https://example.com','US')
        for host in ('www.example.com','news.example.com'):
            sid=self.db.add_source(cid,url=f'https://{host}/event',text='Seeking trash can suppliers. October 1, 2026',retrieved_at='2026-11-01T00:00:00Z')
            c=self.db.add_claim(cid,sid,'procurement','Seeking trash can suppliers.',value='same-event',event_date='2026-10-01',date_quote='October 1, 2026')
            self.db.review_claim(c,'verified','Tester','Synthetic')
        s=score(self.db,cid,AS_OF)
        self.assertFalse(s['verified_procurement'])
        self.assertEqual(s['breakdown']['recent_signal'],1)

    def test_external_procurement_needs_explicit_independence_review(self):
        cid=self.db.upsert_company('Example','https://example.com','US')
        claims=[]
        for host in ('example.com','independent.example.org'):
            sid=self.db.add_source(cid,url=f'https://{host}/event',text='Seeking trash can suppliers. October 1, 2026',retrieved_at='2026-11-01T00:00:00Z')
            claim=self.db.add_claim(cid,sid,'procurement','Seeking trash can suppliers.',value='event-1',event_date='2026-10-01',date_quote='October 1, 2026')
            self.db.review_claim(claim,'verified','Tester','Synthetic')
            claims.append(claim)
        self.assertFalse(score(self.db,cid,AS_OF)['verified_procurement'])
        self.db.review_claim(claims[1],'verified','Tester','Publisher identity checked: independent synthetic source',independent=True)
        self.assertTrue(score(self.db,cid,AS_OF)['verified_procurement'])
        with self.assertRaises(ValueError):
            self.db.review_claim(claims[0],'verified','Tester','First party is not independent',independent=True)

    def test_v1_database_migration_preserves_claims(self):
        import sqlite3
        from eko_prospect.store import SCHEMA
        with tempfile.TemporaryDirectory() as tmp:
            path=Path(tmp)/'v1.sqlite'
            conn=sqlite3.connect(path)
            legacy=SCHEMA.replace(' independent INTEGER NOT NULL DEFAULT 0,','')
            conn.executescript(legacy)
            conn.execute("INSERT INTO companies(domain,market,name,website,group_key) VALUES('example.com','US','Example','https://example.com','example.com')")
            conn.execute('PRAGMA user_version=1')
            conn.commit(); conn.close()
            migrated=Store(path)
            try:
                self.assertEqual(migrated.company(1)['name'],'Example')
                self.assertEqual(migrated.conn.execute('PRAGMA user_version').fetchone()[0],2)
                self.assertIn('independent',{r['name'] for r in migrated.conn.execute('PRAGMA table_info(claims)')})
            finally:
                migrated.close()

    def test_supplier_value_cannot_bind_unquoted_recipient(self):
        cid=qualified(self.db)
        for e in self.db.evidence(cid):
            if e['kind']=='supplier_access':
                self.db.review_claim(e['id'],'rejected','Tester','Replace attribution')
        sid=self.db.add_source(cid,url='https://example.com/portal',text='Supplier application portal.',retrieved_at='2026-11-01T00:00:00Z')
        with self.assertRaises(ValueError):
            self.db.add_claim(cid,sid,'supplier_access','Supplier application portal.',value='vendor@example.com')

    def test_linkedin_requires_complete_identity(self):
        cid=self.db.upsert_company('Example','https://example.com','US')
        url='https://www.linkedin.com/company/acme-other'
        sid=self.db.add_source(cid,url='https://example.com/contact',text=url+'.',retrieved_at='2026-11-01T00:00:00Z')
        with self.assertRaises(ValueError):
            self.db.add_claim(cid,sid,'linkedin',url+'.',value='https://www.linkedin.com/company/acme')
        valid=self.db.add_claim(cid,sid,'linkedin',url+'.',value=url)
        self.assertEqual(self.db.claim(valid)['value'],url)

    def test_stale_review_cannot_approve_new_revision(self):
        cid=qualified(self.db)
        seen=self.db.company(cid)['revision']
        self.db.add_source(cid,url='https://example.com/new',text='New information.',retrieved_at='2026-11-01T00:00:00Z')
        with self.assertRaises(ValueError):
            self.db.review_company(cid,'approved','Tester','Old browser form',relationship='new',angle='Recommendation',as_of=AS_OF,expected_revision=seen)
        self.assertFalse(score(self.db,cid,AS_OF)['qualified'])

    def test_charset_failure_recorded_and_next_source_collected(self):
        cid=self.db.upsert_company('Example','https://example.com','US')
        t=FakeTransport({'https://example.com/robots.txt':Response(404,{},b''),
                        'https://example.com/bad':Response(200,{'content-type':'text/html; charset=not-an-encoding'},b'<p>Trash cans</p>'),
                        'https://example.com/good':html('<p>Trash cans</p>')})
        r=collect(self.db,cid,['https://example.com/bad','https://example.com/good'],Fetcher(t,delay=0))
        self.assertEqual((r['ok'],r['failed']),(1,1))
        self.assertEqual(len(self.db.rows('SELECT * FROM sources')),2)

    def test_contact_excerpts_preserve_original_punctuation(self):
        cid=self.db.upsert_company('Example','https://example.com','US')
        text='Contact our vendor team at vendors@example.com.'
        sid=self.db.add_source(cid,url='https://example.com/contact',text=text,retrieved_at='2026-11-01T00:00:00Z')
        for p in RuleExtractor().propose(text,'US'):
            self.assertIn(p['quote'],text)
            self.db.add_claim(cid,sid,**p)
        self.assertEqual(self.db.contacts(cid)[0]['value'],'vendors@example.com')

    def test_large_malformed_email_token_has_bounded_processing(self):
        code="from eko_prospect.extract import RuleExtractor; assert RuleExtractor().propose('a'*200000+'@'+'b'*200000,'US') == []"
        try:
            result=subprocess.run([sys.executable,'-c',code],capture_output=True,text=True,timeout=3)
        except subprocess.TimeoutExpired:
            self.fail('Malformed public text caused unbounded contact extraction')
        self.assertEqual(result.returncode,0,result.stderr)

    def test_proxy_never_honors_no_proxy_bypass(self):
        with patch.dict(os.environ,{'NO_PROXY':'*','no_proxy':'*'}):
            t=TrustedProxyTransport(proxies={'http':'http://proxy.example:8080','https':'http://proxy.example:8080'})
            handler=next(h for h in t.opener.handlers if isinstance(h,urllib.request.ProxyHandler))
            request=urllib.request.Request('https://example.com/')
            handler.proxy_open(request,'http://proxy.example:8080','https')
            self.assertEqual(request.host,'proxy.example:8080')
            self.assertEqual(request._tunnel_host,'example.com')

    def test_proxy_missing_scheme_fails_closed(self):
        t=TrustedProxyTransport(proxies={'http':'http://proxy.example:8080'})
        with patch.object(t.opener,'open',side_effect=AssertionError('Must not attempt a direct connection')):
            with self.assertRaises(ValueError):
                t.get('https://example.com/')

    def test_same_group_distinct_emails_block_duplicate_draft(self):
        a=qualified(self.db)
        b=qualified(self.db,'other.com','vendor@other.com')
        self.db.review_company(b,'approved','Tester','Same group',relationship='new',group_key='example.com',angle='Recommendation',as_of=AS_OF,expected_revision=self.db.company(b)['revision'])
        prepare_draft(self.db,a,'vendor@example.com','Hello','Text','Tester','vendor@example.com',AS_OF)
        with self.assertRaises(ValueError):
            prepare_draft(self.db,b,'vendor@other.com','Hello','Text','Tester','vendor@other.com',AS_OF)


if __name__=='__main__': unittest.main()
