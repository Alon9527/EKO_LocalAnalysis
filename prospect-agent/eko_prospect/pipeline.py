"""Bounded collection and strict import orchestration."""
from .extract import RuleExtractor
from .fetch import Fetcher
from .identity import canonical_url


def collect(db, cid, urls, fetcher=None, extractor=None, max_pages=5, follow_links=False):
    if not 1 <= max_pages <= 20:
        raise ValueError('max_pages must be 1..20')
    fetcher = fetcher or Fetcher()
    extractor = extractor or RuleExtractor()
    company = db.company(cid)
    unique = list(dict.fromkeys(canonical_url(u) for u in urls))[:max_pages]
    stats = {'company_id':cid,'ok':0,'failed':0,'source_ids':[],'claim_ids':[],'errors':[]}
    visited=set()
    while unique and len(visited)<max_pages:
        url=unique.pop(0)
        if url in visited:
            continue
        visited.add(url)
        doc = fetcher.fetch(url)
        links=doc.pop('links',[])
        if follow_links:
            from urllib.parse import urlsplit
            import re
            relevant=[u for u in links if re.search(r'supplier|vendor|contact|about|news|trash|waste|recycling',urlsplit(u).path,re.I)
                      and not re.search(r'\.(pdf|zip|png|jpe?g|webp|mp4|docx?|xlsx?)$',urlsplit(u).path,re.I)]
            relevant.sort(key=lambda u: (0 if re.search(r'supplier|vendor|trash|waste|recycling',urlsplit(u).path,re.I) else 1,u))
            unique.extend(u for u in relevant if u not in visited and u not in unique)
        sid = db.add_source(cid, **doc)
        stats['source_ids'].append(sid)
        with db.tx():
            db.audit('system','collection_attempt',cid,{'requested_url':url,'source_url':doc['url'],'status':doc['status']})
        if doc['status'] != 'ok':
            stats['failed'] += 1
            stats['errors'].append({'url':url,'error':doc['error']})
            continue
        stats['ok'] += 1
        for proposal in extractor.propose(doc['text'], company['market']):
            try:
                stats['claim_ids'].append(db.add_claim(cid,sid,**proposal))
            except ValueError as exc:
                stats['errors'].append({'url':url,'error':str(exc)})
    return stats


def import_bundle(db, bundle):
    if set(bundle) - {'companies','description','schema_version'}:
        raise ValueError('Unknown bundle field')
    companies = bundle.get('companies')
    if not isinstance(companies,list) or len(companies)>100:
        raise ValueError('Bundle requires <=100 companies')
    ids=[]
    with db.tx():
        for company in companies:
            if set(company)-{'name','website','market','customer_type','sources'}:
                raise ValueError('Unknown company field; review state cannot be imported')
            cid=db.upsert_company(company['name'],company['website'],company.get('market','US'),company.get('customer_type',''))
            ids.append(cid)
            for source in company.get('sources',[]):
                if set(source)-{'url','text','retrieved_at','captured_by','claims'}:
                    raise ValueError('Unknown source field')
                sid=db.add_source(cid,url=source['url'],text=source['text'],retrieved_at=source['retrieved_at'],
                                  method='manual_excerpt',captured_by=source['captured_by'])
                for claim in source.get('claims',[]):
                    if set(claim)-{'kind','quote','value','event_date','date_quote'}:
                        raise ValueError('Unknown claim field; verified status cannot be imported')
                    db.add_claim(cid,sid,**claim)
    return {'companies':ids, 'note':'All imported evidence is pending human review.'}
