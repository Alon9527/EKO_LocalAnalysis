"""Explainable scoring; evidence of expansion is never evidence of purchase."""
from datetime import date
from .identity import months_before, timestamp

RUBRIC_VERSION = 'eko-v1.0'


def usable(e, as_of):
    observed = timestamp(e['retrieved_at']).date()
    return (e['status'] == 'verified' and e['source_status'] == 'ok' and e['active']
            and months_before(as_of) <= observed <= as_of)


def score(db, cid, as_of: date):
    company = db.company(cid)
    evidence = db.evidence(cid)
    verified = [e for e in evidence if usable(e, as_of)]
    primary = [e for e in verified if e['first_party']]
    by = lambda kind: [e for e in primary if e['kind'] == kind]
    recent = [e for e in primary if e['kind'] in ('expansion', 'procurement')
              and e['event_date'] and months_before(as_of) <= date.fromisoformat(e['event_date']) <= as_of]
    procurement = [e for e in recent if e['kind'] == 'procurement']
    # Require an independent host to corroborate the same dated procurement event.
    from urllib.parse import urlsplit
    def corroborated(e):
        return any(other['kind'] == 'procurement' and not other['first_party'] and other['independent']
                   and other['event_date'] == e['event_date']
                   and other['value'] and other['value'] == e['value']
                   and urlsplit(other['url']).hostname != urlsplit(e['url']).hostname
                   for other in verified)
    confirmed_purchase = any(corroborated(e) for e in procurement)
    breakdown = {'category': 2 if by('category') else 0, 'channel': 2 if by('channel') else 0,
                 'recent_signal': 2 if confirmed_purchase else (1 if recent else 0),
                 'supplier_access': 2 if by('supplier_access') else 0,
                 'commercial_fit': 2 if by('commercial_fit') else 0}
    raw = sum(breakdown.values())
    blockers = []
    if not by('market'):
        blockers.append('market_unverified')
    if not by('category'):
        blockers.append('category_unverified')
    if not by('supplier_access'):
        blockers.append('supplier_route_missing')
    if len({e['url'] for e in verified}) < 2:
        blockers.append('need_two_source_urls')
    if company['relationship'] == 'unknown':
        blockers.append('existing_relationship_unchecked')
    if company['relationship'] == 'do_not_contact':
        blockers.append('do_not_contact')
    if db.rows('SELECT key FROM suppressions WHERE key IN (?,?)', (company['domain'], company['group_key'])):
        blockers.append('suppressed')
    if raw < 8:
        blockers.append('score_below_8')
    approved = company['review_status'] == 'approved' and company['reviewed_revision'] == company['revision']
    missing = [k for k in ('market', 'category', 'channel', 'supplier_access', 'commercial_fit', 'email', 'linkedin') if not by(k)]
    if not recent:
        missing.append('recent_dated_signal')
    return {'score': max(1, raw), 'raw_total': raw, 'breakdown': breakdown, 'rubric_version': RUBRIC_VERSION,
            'as_of': as_of.isoformat(), 'window_start': months_before(as_of).isoformat(),
            'verified_procurement': confirmed_purchase, 'missing': missing, 'blockers': blockers,
            'qualified': approved and not blockers, 'verified_claim_ids': [e['id'] for e in verified],
            'pending_claims': sum(e['status'] == 'pending' for e in evidence),
            'stale_or_inactive_claims': sum(e['status'] == 'verified' and not usable(e, as_of) for e in evidence)}
