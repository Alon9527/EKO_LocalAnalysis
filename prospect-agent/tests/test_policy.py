import tempfile
import unittest
from datetime import date
from pathlib import Path

from eko_prospect.identity import canonical_url, company_key, email_key, months_before
from eko_prospect.store import Store
from eko_prospect.policy import score


class IdentityTests(unittest.TestCase):
    def test_domain_email_and_url_dedup(self):
        self.assertEqual(company_key('https://WWW.Example.com/shop?x=1'), 'example.com')
        self.assertNotEqual(company_key('https://shop.example.com'), company_key('https://example.com'))
        self.assertEqual(email_key(' Sales@Example.COM '), 'sales@example.com')
        self.assertNotEqual(email_key('a.b@example.com'), email_key('ab@example.com'))
        self.assertEqual(canonical_url('https://www.example.com/a?utm_source=x&sku=1#x'), 'https://www.example.com/a?sku=1')

    def test_calendar_six_months_boundary(self):
        self.assertEqual(months_before(date(2026, 8, 31)), date(2026, 2, 28))
        self.assertEqual(months_before(date(2026, 11, 1)), date(2026, 5, 1))


class PolicyTests(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        self.db = Store(Path(self.tmp.name) / 'db.sqlite')
        self.company = self.db.upsert_company('Acme', 'https://example.com', 'US')
        self.source = self.db.add_source(self.company, url='https://example.com/about', text='United States. Trash cans. Wholesale distributor. Supplier application. October 1, 2026. Opened a new distribution center.', retrieved_at='2026-11-01T08:00:00+00:00')

    def tearDown(self):
        self.db.close()
        self.tmp.cleanup()

    def claim(self, kind, quote, **kw):
        return self.db.add_claim(self.company, self.source, kind, quote, **kw)

    def verify(self, cid):
        self.db.review_claim(cid, 'verified', 'Alon', '原文和语义已逐项核对')

    def test_proposals_never_score_and_quote_must_exist(self):
        c = self.claim('category', 'Trash cans.')
        self.assertEqual(score(self.db, self.company, date(2026, 11, 1))['raw_total'], 0)
        with self.assertRaises(ValueError):
            self.claim('procurement', 'seeking EKO suppliers')
        self.verify(c)
        self.assertEqual(score(self.db, self.company, date(2026, 11, 1))['breakdown']['category'], 2)

    def test_event_requires_date_anchor_and_does_not_mean_purchase(self):
        with self.assertRaises(ValueError):
            self.claim('expansion', 'Opened a new distribution center.', event_date='2026-10-01')
        c = self.claim('expansion', 'Opened a new distribution center.', event_date='2026-10-01', date_quote='October 1, 2026')
        self.verify(c)
        s = score(self.db, self.company, date(2026, 11, 1))
        self.assertEqual(s['breakdown']['recent_signal'], 1)
        self.assertFalse(s['verified_procurement'])
        self.assertEqual(score(self.db, self.company, date(2027, 5, 1))['breakdown']['recent_signal'], 0)

    def test_future_date_not_recent(self):
        c = self.claim('expansion', 'Opened a new distribution center.', event_date='2026-10-01', date_quote='October 1, 2026')
        self.verify(c)
        self.assertEqual(score(self.db, self.company, date(2026, 9, 30))['breakdown']['recent_signal'], 0)

    def test_cross_company_source_rejected(self):
        b = self.db.upsert_company('Other', 'https://other.com', 'US')
        with self.assertRaises(ValueError):
            self.db.add_claim(b, self.source, 'category', 'Trash cans.')

    def test_dedup_idempotent_and_review_invalidated_on_change(self):
        self.assertEqual(self.company, self.db.upsert_company('Acme', 'https://www.example.com/', 'US'))
        c = self.claim('category', 'Trash cans.')
        self.verify(c)
        self.db.review_company(self.company, 'hold', 'Alon', '需要补充', relationship='new')
        revision = self.db.company(self.company)['revision']
        self.assertEqual(c, self.claim('category', 'Trash cans.'))
        self.assertEqual(self.db.company(self.company)['revision'], revision)
        self.assertEqual(self.db.claim(c)['status'], 'verified')
        self.claim('channel', 'Wholesale distributor.')
        self.assertEqual(self.db.company(self.company)['review_status'], 'pending')

    def test_email_requires_exact_public_observation_and_dedups(self):
        with self.assertRaises(ValueError):
            self.db.add_claim(self.company, self.source, 'email', 'Trash cans.', value='guessed@example.com')
        s = self.db.add_source(self.company, url='https://example.com/contact', text='Supplier team: SALES@Example.com', retrieved_at='2026-11-01T08:00:00+00:00')
        a = self.db.add_claim(self.company, s, 'email', 'Supplier team: SALES@Example.com', value='sales@example.com')
        self.verify(a)
        self.assertEqual(len(self.db.contacts(self.company)), 1)

    def test_reviewer_and_reason_required(self):
        c = self.claim('category', 'Trash cans.')
        with self.assertRaises(ValueError):
            self.db.review_claim(c, 'verified', '', '')

    def test_latest_source_change_revokes_old_claim_points(self):
        c = self.claim('category', 'Trash cans.')
        self.verify(c)
        self.db.add_source(self.company, url='https://example.com/about', text='Furniture only.', retrieved_at='2026-11-02T08:00:00+00:00')
        self.assertEqual(score(self.db, self.company, date(2026, 11, 2))['breakdown']['category'], 0)


if __name__ == '__main__':
    unittest.main()
