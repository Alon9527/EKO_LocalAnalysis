import json
import tempfile
import unittest
from datetime import date
from pathlib import Path

from eko_prospect.fetch import Fetcher, Response, UnsafeURL, public_addresses, html_text
from eko_prospect.extract import RuleExtractor
from eko_prospect.pipeline import collect, import_bundle
from eko_prospect.providers import CsvSearchProvider, discover, queries
from eko_prospect.store import Store


class FakeTransport:
    def __init__(self, pages):
        self.pages = pages
        self.visits = []

    def get(self, url):
        self.visits.append(url)
        return self.pages.get(url, Response(404, {}, b''))


def html(text):
    return Response(200, {'content-type': 'text/html; charset=utf-8'}, text.encode())


class FetchTests(unittest.TestCase):
    def test_denies_private_loopback_and_metadata(self):
        for url in ('http://127.0.0.1/', 'http://169.254.169.254/', 'http://[::1]/', 'file:///etc/passwd'):
            with self.subTest(url=url), self.assertRaises((UnsafeURL, ValueError)):
                public_addresses(url)

    def test_robots_denial_is_not_bypassed(self):
        t = FakeTransport({'https://example.com/robots.txt': Response(200, {}, b'User-agent: *\nDisallow: /private')})
        r = Fetcher(t, delay=0).fetch('https://example.com/private')
        self.assertEqual(r['status'], 'blocked')
        self.assertNotIn('https://example.com/private', t.visits)

    def test_redirect_is_validated_before_following(self):
        t = FakeTransport({'https://example.com/robots.txt': Response(404, {}, b''),
                           'https://example.com/': Response(302, {'location':'http://127.0.0.1/'}, b'')})
        r = Fetcher(t, delay=0).fetch('https://example.com/')
        self.assertEqual(r['status'], 'blocked')
        self.assertNotIn('http://127.0.0.1/', t.visits)

    def test_cross_host_redirect_checks_destination_robots(self):
        t = FakeTransport({'https://example.com/robots.txt': Response(404, {}, b''),
                           'https://example.com/': Response(302, {'location':'https://example.org/private'}, b''),
                           'https://example.org/robots.txt': Response(200, {}, b'User-agent: *\nDisallow: /')})
        r = Fetcher(t, delay=0).fetch('https://example.com/')
        self.assertEqual(r['status'], 'blocked')
        self.assertNotIn('https://example.org/private', t.visits)

    def test_script_hidden_text_excluded_and_link_visible(self):
        text = html_text('<script>buyer@x.com</script><style>x</style><p>Trash cans</p><a href="mailto:vendor@example.com">Email team</a>')
        self.assertNotIn('buyer@x.com', text)
        self.assertIn('vendor@example.com', text)

    def test_retry_503_then_record_error(self):
        t = FakeTransport({'https://example.com/robots.txt': Response(404, {}, b''), 'https://example.com/': Response(503, {}, b'')})
        r = Fetcher(t, delay=0, retries=1, sleeper=lambda _:None).fetch('https://example.com/')
        self.assertEqual(r['status'], 'error')
        self.assertEqual(t.visits.count('https://example.com/'), 2)

    def test_html_content_type_and_size_rejected(self):
        for response in (Response(200, {'content-type':'application/pdf'}, b'abc'), html('x'*2_000_001)):
            t = FakeTransport({'https://example.com/robots.txt': Response(404, {}, b''),'https://example.com/': response})
            self.assertEqual(Fetcher(t, delay=0).fetch('https://example.com/')['status'], 'error')


class PipelineTests(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        self.db = Store(Path(self.tmp.name)/'state.db')
        self.cid = self.db.upsert_company('Example','https://example.com','US')

    def tearDown(self):
        self.db.close()
        self.tmp.cleanup()

    def test_extractor_does_not_invent_events_or_people(self):
        proposals = RuleExtractor().propose('Trash cans. Supplier application: vendor@example.com. Copyright 2026. We opened 100 stores.', 'US')
        self.assertTrue(any(p['kind']=='category' for p in proposals))
        self.assertTrue(any(p['kind']=='email' and p['value']=='vendor@example.com' for p in proposals))
        self.assertFalse(any(p['kind'] in ('procurement','expansion') for p in proposals))

    def test_collect_failure_and_success_survive(self):
        t = FakeTransport({'https://example.com/robots.txt': Response(404, {}, b''),
                           'https://example.com/': html('<h1>Trash cans</h1><p>vendor@example.com</p>')})
        result = collect(self.db,self.cid,['https://example.com/','https://example.com/nope'],Fetcher(t,delay=0))
        self.assertEqual(result['ok'],1)
        self.assertEqual(result['failed'],1)
        self.assertTrue(all(e['status']=='pending' for e in self.db.evidence(self.cid)))
        self.assertEqual(len(self.db.rows('SELECT * FROM sources')),2)

    def test_invalid_bundle_atomic_and_no_injected_review(self):
        bundle = {'companies':[{'name':'Acme','website':'https://acme.com','market':'US','sources':[
            {'url':'https://acme.com','text':'Trash cans.','retrieved_at':'2026-11-01T00:00:00Z','captured_by':'Researcher','claims':[
                {'kind':'category','quote':'Trash cans.','status':'verified'}]}]}]}
        with self.assertRaises(ValueError):
            import_bundle(self.db,bundle)
        self.assertEqual(len(self.db.rows('SELECT * FROM companies')),1)

    def test_discover_not_filled_to_twenty(self):
        p = Path(self.tmp.name)/'seeds.csv'
        p.write_text('name,website,market,customer_type\nExample,https://example.com/,US,distributor\nDuplicate,https://www.example.com/,US,distributor\n',encoding='utf-8')
        stats = discover(self.db,CsvSearchProvider(p),'US',limit=20)
        self.assertEqual(stats['unique_candidates'],1)
        self.assertEqual(stats['target'],20)
        self.assertIn('OEM', ' '.join(queries('US')))
        self.assertIn('Germany', ' '.join(queries('DE')))


if __name__ == '__main__':
    unittest.main()
