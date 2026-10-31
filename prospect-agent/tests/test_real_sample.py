"""Historical, reproducible E2E with real short excerpts; no customer is actually approved."""
from datetime import date
import json
from pathlib import Path
import tempfile
import unittest
from eko_prospect.store import Store
from eko_prospect.pipeline import import_bundle
from eko_prospect.policy import score
from eko_prospect.export import export_csv,portfolio


class RealSampleTests(unittest.TestCase):
    def test_real_sources_are_traceable_and_expansion_is_not_purchase(self):
        path=Path(__file__).resolve().parents[1]/'examples/real_public_sample.json'
        with tempfile.TemporaryDirectory() as tmp:
            db=Store(Path(tmp)/'sample.sqlite')
            try:
                result=import_bundle(db,json.loads(path.read_text(encoding='utf-8')))
                self.assertEqual(len(result['companies']),3)
                as_of=date(2026,11,1)
                self.assertEqual(portfolio(db,as_of)['qualified'],0)
                for cid in result['companies']:
                    self.assertEqual(score(db,cid,as_of)['raw_total'],0)
                    for e in db.evidence(cid):
                        self.assertTrue(e['url'].startswith('https://'))
                        self.assertEqual(len(e['sha256']),64)
                        # Simulated review in this temporary test DB; never applied to delivered sample DB.
                        db.review_claim(e['id'],'verified','automated-test-only','Exercise human-review code path on published excerpts')
                    db.review_company(cid,'hold','automated-test-only','Missing evidence and relationship check',as_of=as_of)
                self.assertEqual(score(db,1,as_of)['raw_total'],6)
                self.assertEqual(score(db,2,as_of)['raw_total'],4)
                self.assertEqual(score(db,3,as_of)['raw_total'],5)
                self.assertFalse(score(db,3,as_of)['verified_procurement'])
                self.assertEqual(score(db,3,as_of)['breakdown']['recent_signal'],1)
                self.assertEqual(portfolio(db,as_of)['qualified'],0)
                self.assertEqual([c['value'] for c in db.contacts(1) if c['kind']=='email'],['vendors@katom.com'])
                self.assertEqual(db.contacts(2),[])
                export_csv(db,Path(tmp)/'real.csv',as_of)
                self.assertIn('recent_signal',(Path(tmp)/'real.csv').read_text(encoding='utf-8-sig'))
                self.assertEqual(score(db,1,date(2027,6,1))['raw_total'],0)
            finally:
                db.close()
