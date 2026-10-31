"""SQLite provenance, review audit, contacts and durable outreach reservations."""
from contextlib import contextmanager
import hashlib
import json
import sqlite3
from datetime import date
from pathlib import Path
from urllib.parse import urlsplit

from .identity import canonical_url, company_key, email_key, linkedin_key, norm, parse_date, timestamp, utc_now, public_emails, linkedin_urls

KINDS = {'market', 'category', 'channel', 'supplier_access', 'expansion', 'procurement',
         'commercial_fit', 'email', 'linkedin', 'existing_listing'}
EVENT_KINDS = {'expansion', 'procurement'}

SCHEMA = '''
CREATE TABLE IF NOT EXISTS companies (
 id INTEGER PRIMARY KEY, domain TEXT NOT NULL, market TEXT NOT NULL, name TEXT NOT NULL,
 website TEXT NOT NULL, group_key TEXT NOT NULL, customer_type TEXT NOT NULL DEFAULT '',
 relationship TEXT NOT NULL DEFAULT 'unknown', review_status TEXT NOT NULL DEFAULT 'pending',
 revision INTEGER NOT NULL DEFAULT 1, reviewed_revision INTEGER, reviewer TEXT, reviewed_at TEXT,
 review_note TEXT, angle TEXT NOT NULL DEFAULT '', UNIQUE(domain,market));
CREATE TABLE IF NOT EXISTS sources (
 id INTEGER PRIMARY KEY, company_id INTEGER NOT NULL REFERENCES companies(id), url TEXT NOT NULL,
 text TEXT NOT NULL, sha256 TEXT NOT NULL, retrieved_at TEXT NOT NULL, status TEXT NOT NULL,
 method TEXT NOT NULL, captured_by TEXT NOT NULL, first_party INTEGER NOT NULL,
 error TEXT NOT NULL, active INTEGER NOT NULL DEFAULT 1,
 UNIQUE(company_id,url,sha256,retrieved_at,status));
CREATE TABLE IF NOT EXISTS claims (
 id INTEGER PRIMARY KEY, company_id INTEGER NOT NULL REFERENCES companies(id),
 source_id INTEGER NOT NULL REFERENCES sources(id), kind TEXT NOT NULL, value TEXT NOT NULL,
 quote TEXT NOT NULL, event_date TEXT, date_quote TEXT NOT NULL DEFAULT '',
 status TEXT NOT NULL DEFAULT 'pending', reviewer TEXT, reviewed_at TEXT, review_note TEXT,
 independent INTEGER NOT NULL DEFAULT 0,
 UNIQUE(company_id,source_id,kind,value,quote,event_date));
CREATE TABLE IF NOT EXISTS audit (
 id INTEGER PRIMARY KEY, at TEXT NOT NULL, actor TEXT NOT NULL, action TEXT NOT NULL,
 entity TEXT NOT NULL, details TEXT NOT NULL);
CREATE TABLE IF NOT EXISTS suppressions (
 key TEXT PRIMARY KEY, reason TEXT NOT NULL, actor TEXT NOT NULL, at TEXT NOT NULL);
CREATE TABLE IF NOT EXISTS outreach (
 id INTEGER PRIMARY KEY, company_id INTEGER NOT NULL REFERENCES companies(id),
 group_key TEXT NOT NULL UNIQUE, email TEXT NOT NULL UNIQUE, subject TEXT NOT NULL,
 body TEXT NOT NULL, body_sha256 TEXT NOT NULL, confirmed_by TEXT NOT NULL,
 confirmed_at TEXT NOT NULL, status TEXT NOT NULL DEFAULT 'prepared', note TEXT NOT NULL DEFAULT '');
'''


class Store:
    def __init__(self, path):
        self.path = str(path)
        if self.path != ':memory:':
            Path(path).parent.mkdir(parents=True, exist_ok=True)
        self.conn = sqlite3.connect(self.path, timeout=15)
        self.conn.row_factory = sqlite3.Row
        self.conn.execute('PRAGMA foreign_keys=ON')
        version = self.conn.execute('PRAGMA user_version').fetchone()[0]
        if version not in (0, 1, 2):
            raise ValueError(f'Unsupported DB version: {version}')
        self.conn.executescript(SCHEMA)
        columns = {row['name'] for row in self.conn.execute('PRAGMA table_info(claims)')}
        if 'independent' not in columns:
            self.conn.execute('ALTER TABLE claims ADD COLUMN independent INTEGER NOT NULL DEFAULT 0')
        self.conn.execute('PRAGMA user_version=2')

    @contextmanager
    def tx(self):
        # Savepoints compose so bundle imports are atomic even when methods nest.
        import uuid
        name = 'tx_' + uuid.uuid4().hex
        self.conn.execute('SAVEPOINT ' + name)
        try:
            yield
        except BaseException:
            self.conn.execute('ROLLBACK TO ' + name)
            self.conn.execute('RELEASE ' + name)
            raise
        else:
            self.conn.execute('RELEASE ' + name)

    def close(self):
        self.conn.close()

    def rows(self, sql, args=()):
        return [dict(r) for r in self.conn.execute(sql, args)]

    def company(self, cid):
        rows = self.rows('SELECT * FROM companies WHERE id=?', (cid,))
        if not rows:
            raise ValueError('Company not found')
        return rows[0]

    def claim(self, cid):
        rows = self.rows('SELECT * FROM claims WHERE id=?', (cid,))
        if not rows:
            raise ValueError('Claim not found')
        return rows[0]

    def source(self, sid):
        rows = self.rows('SELECT * FROM sources WHERE id=?', (sid,))
        if not rows:
            raise ValueError('Source not found')
        return rows[0]

    def audit(self, actor, action, entity, details):
        self.conn.execute('INSERT INTO audit(at,actor,action,entity,details) VALUES(?,?,?,?,?)',
                          (utc_now(), actor, action, str(entity), json.dumps(details, ensure_ascii=False)))

    def touch(self, cid):
        self.conn.execute("UPDATE companies SET revision=revision+1,review_status='pending' WHERE id=?", (cid,))

    def upsert_company(self, name, website, market='US', customer_type=''):
        if not name.strip() or market not in ('US', 'CA', 'GB', 'DE'):
            raise ValueError('Name and supported market required')
        domain = company_key(website)
        with self.tx():
            existing = self.rows('SELECT id FROM companies WHERE domain=? AND market=?', (domain, market))
            if existing:
                return existing[0]['id']
            cur = self.conn.execute('INSERT INTO companies(domain,market,name,website,group_key,customer_type) VALUES(?,?,?,?,?,?)',
                                    (domain, market, name.strip(), canonical_url(website), domain, customer_type))
            self.audit('system', 'company_discovered', cur.lastrowid, {'website': website, 'market': market})
            return cur.lastrowid

    def add_source(self, cid, *, url, text='', retrieved_at=None, status='ok', method='http', captured_by='', error=''):
        company = self.company(cid)
        url = canonical_url(url)
        retrieved_at = retrieved_at or utc_now()
        timestamp(retrieved_at)
        if status not in ('ok', 'blocked', 'error') or method not in ('http', 'manual_excerpt'):
            raise ValueError('Invalid capture metadata')
        if method == 'manual_excerpt' and not captured_by.strip():
            raise ValueError('Manual excerpts require captured_by')
        if status == 'ok' and not text.strip():
            raise ValueError('Successful source requires text')
        digest = hashlib.sha256(text.encode()).hexdigest()
        domain = company_key(url)
        first_party = domain == company['domain'] or domain.endswith('.' + company['domain'])
        with self.tx():
            same = self.rows('SELECT id FROM sources WHERE company_id=? AND url=? AND sha256=? AND retrieved_at=? AND status=?',
                             (cid, url, digest, retrieved_at, status))
            if same:
                return same[0]['id']
            # Older captures remain auditable, never replace a newer observation.
            latest = self.rows('SELECT * FROM sources WHERE company_id=? AND url=? AND active=1', (cid, url))
            active = not latest or timestamp(retrieved_at) >= max(timestamp(s['retrieved_at']) for s in latest)
            if active:
                self.conn.execute('UPDATE sources SET active=0 WHERE company_id=? AND url=?', (cid, url))
            cur = self.conn.execute('INSERT INTO sources(company_id,url,text,sha256,retrieved_at,status,method,captured_by,first_party,error,active) VALUES(?,?,?,?,?,?,?,?,?,?,?)',
                (cid, url, text, digest, retrieved_at, status, method, captured_by, int(first_party), error, int(active)))
            self.touch(cid)
            self.audit('system', 'source_recorded', cur.lastrowid, {'url': url, 'status': status, 'method': method})
            return cur.lastrowid

    def add_claim(self, cid, sid, kind, quote, *, value='', event_date=None, date_quote=''):
        source = self.source(sid)
        quote = norm(quote)
        if source['company_id'] != cid or source['status'] != 'ok':
            raise ValueError('Source must be successful and belong to company')
        if kind not in KINDS or not quote or len(quote.split()) > 25 or norm(quote).casefold() not in norm(source['text']).casefold():
            raise ValueError('Claim requires a supported kind and an exact source excerpt (<=25 words)')
        if kind in EVENT_KINDS:
            if not event_date or not date_quote or norm(date_quote) not in norm(source['text']):
                raise ValueError('Events require event date and exact date quote, not retrieval date')
            if parse_date(date_quote) != date.fromisoformat(event_date):
                raise ValueError('Date quote must unambiguously match event_date')
        elif event_date is not None:
            raise ValueError('Only events may have event_date')
        if kind == 'email':
            value = email_key(value)
            if value not in public_emails(quote):
                raise ValueError('Email must be explicitly visible in the source excerpt')
        if kind == 'supplier_access' and '@' in value:
            value = email_key(value)
            if value not in public_emails(quote):
                raise ValueError('Supplier email must occur exactly in the supplier-purpose excerpt')
        if kind == 'linkedin':
            value = linkedin_key(value)
            if value not in linkedin_urls(quote):
                raise ValueError('Complete LinkedIn identity must appear in excerpt')
        if kind == 'market' and value != self.company(cid)['market']:
            raise ValueError('Market claim must match target market; reviewer verifies market meaning')
        with self.tx():
            same = self.rows('SELECT id FROM claims WHERE company_id=? AND source_id=? AND kind=? AND value=? AND quote=? AND event_date IS ?',
                             (cid, sid, kind, value, quote, event_date))
            if same:
                return same[0]['id']
            cur = self.conn.execute('INSERT INTO claims(company_id,source_id,kind,value,quote,event_date,date_quote) VALUES(?,?,?,?,?,?,?)',
                                    (cid, sid, kind, value, quote, event_date, date_quote))
            self.touch(cid)
            self.audit('system', 'claim_proposed', cur.lastrowid, {'kind': kind})
            return cur.lastrowid

    @staticmethod
    def check_review(actor, note):
        if not actor.strip() or not note.strip():
            raise ValueError('Human reviewer name and review reason are required')

    def review_claim(self, claim_id, status, actor, note, *, independent=False):
        self.check_review(actor, note)
        if status not in ('verified', 'rejected', 'pending'):
            raise ValueError('Invalid claim status')
        c = self.claim(claim_id)
        s = self.source(c['source_id'])
        if status == 'verified' and (s['status'] != 'ok' or not s['active']):
            raise ValueError('Cannot verify an inactive/failed source; collect current evidence')
        if independent and (c['kind'] != 'procurement' or s['first_party'] or status != 'verified'):
            raise ValueError('Independent corroboration requires a verified external procurement source')
        with self.tx():
            self.conn.execute('UPDATE claims SET status=?,reviewer=?,review_note=?,reviewed_at=?,independent=? WHERE id=?',
                              (status, actor.strip(), note.strip(), utc_now(), int(independent), claim_id))
            self.touch(c['company_id'])
            self.audit(actor, 'claim_review', claim_id, {'status': status, 'note': note, 'independent': independent})

    def evidence(self, cid):
        return self.rows('''SELECT c.*,s.url,s.sha256,s.retrieved_at,s.status AS source_status,
        s.active,s.first_party,s.method,s.captured_by FROM claims c JOIN sources s ON s.id=c.source_id
        WHERE c.company_id=? ORDER BY c.id''', (cid,))

    def contacts(self, cid):
        # Derived from current observations; no orphaned contact records after evidence withdrawal.
        result = {}
        for c in self.evidence(cid):
            if c['kind'] in ('email', 'linkedin') and c['active'] and c['source_status'] == 'ok' and c['status'] != 'rejected':
                key = (c['kind'], c['value'])
                if key not in result or c['status'] == 'verified':
                    result[key] = {'kind': c['kind'], 'value': c['value'], 'status': c['status'], 'claim_id': c['id'], 'url': c['url'], 'retrieved_at': c['retrieved_at']}
        return list(result.values())

    def review_company(self, cid, status, actor, note, *, relationship='unknown', group_key=None, angle='', as_of=None, expected_revision=None):
        self.check_review(actor, note)
        if status not in ('approved', 'hold', 'rejected') or relationship not in ('unknown', 'new', 'existing', 'do_not_contact'):
            raise ValueError('Invalid review or relationship')
        company = self.company(cid)
        group = group_key.strip().lower() if group_key else company['group_key']
        if not group or len(group) > 200:
            raise ValueError('Invalid group key')
        if status == 'approved' and expected_revision is None:
            raise ValueError('Approval requires expected_revision from the company view; reload before approving')
        expected_revision = company['revision'] if expected_revision is None else expected_revision
        with self.tx():
            updated = self.conn.execute('UPDATE companies SET relationship=?,group_key=?,angle=?,revision=revision+1 WHERE id=? AND revision=?',
                              (relationship, group, angle.strip(), cid, expected_revision))
            if updated.rowcount != 1:
                raise ValueError('Company changed since your review; reload and review the current revision')
            if status == 'approved':
                from .policy import score
                s = score(self, cid, as_of or date.today())
                if s['blockers']:
                    raise ValueError('Approval blocked: ' + '; '.join(s['blockers']))
                if not angle.strip():
                    raise ValueError('EKO cooperation angle required (labelled as recommendation)')
            self.conn.execute('UPDATE companies SET review_status=?,reviewed_revision=revision,reviewer=?,review_note=?,reviewed_at=? WHERE id=?',
                              (status, actor, note, utc_now(), cid))
            self.audit(actor, 'company_review', cid, {'status': status, 'relationship': relationship, 'group_key': group, 'angle': angle, 'note': note})

    def suppress(self, key, reason, actor):
        self.check_review(actor, reason)
        key = key.strip().lower()
        if not key:
            raise ValueError('Suppression key required')
        with self.tx():
            self.conn.execute('INSERT INTO suppressions VALUES(?,?,?,?) ON CONFLICT(key) DO UPDATE SET reason=excluded.reason,actor=excluded.actor,at=excluded.at', (key, reason, actor, utc_now()))
            self.audit(actor, 'suppressed', key, {'reason': reason})
