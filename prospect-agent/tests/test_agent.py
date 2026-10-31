from datetime import date
from pathlib import Path
import tempfile
import unittest
from eko_prospect.agent import run
from eko_prospect.fetch import Fetcher,Response
from eko_prospect.providers import CsvSearchProvider
from eko_prospect.store import Store
from test_acquisition import FakeTransport,html


class AgentTests(unittest.TestCase):
    def test_run_discovers_follows_bounded_links_and_exports(self):
        with tempfile.TemporaryDirectory() as tmp:
            db=Store(Path(tmp)/'db.sqlite')
            try:
                seeds=Path(tmp)/'seeds.csv'
                seeds.write_text('name,website,market\nExample,https://example.com/,US\n')
                t=FakeTransport({'https://example.com/robots.txt':Response(404,{},b''),
                    'https://example.com/':html('<h1>Trash cans</h1><a href="/aaa-vendor.pdf">Vendor PDF</a><a href="/suppliers">Supplier application</a><a href="https://evil.example/contact">Other</a>'),
                    'https://example.com/suppliers':html('<p>vendor@example.com Supplier application</p><a href="/about">About</a>')})
                result=run(db,CsvSearchProvider(seeds),market='US',limit=20,max_pages=2,fetcher=Fetcher(t,delay=0),as_of=date(2026,11,1),out=Path(tmp)/'result.csv')
                self.assertEqual(result['portfolio']['candidates'],1)
                self.assertEqual(result['portfolio']['qualified'],0)
                self.assertTrue((Path(tmp)/'result.csv').exists())
                self.assertIn('https://example.com/suppliers',t.visits)
                self.assertNotIn('https://example.com/about',t.visits)
                self.assertNotIn('https://example.com/aaa-vendor.pdf',t.visits)
                self.assertNotIn('https://evil.example/contact',t.visits)
                self.assertTrue(any(c['value']=='vendor@example.com' for c in db.evidence(1)))
            finally:
                db.close()
