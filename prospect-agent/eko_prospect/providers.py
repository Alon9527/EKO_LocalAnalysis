"""Discovery adapters; search results are leads, never verified facts."""
import csv
import json
import os
from pathlib import Path
from typing import Protocol
from urllib.parse import urlencode, urlsplit
import urllib.request
from .identity import company_key, canonical_url, utc_now

MARKETS = {'US': {'name':'United States','language':'en'},'CA': {'name':'Canada','language':'en'},
           'GB': {'name':'United Kingdom','language':'en'},'DE': {'name':'Germany','language':'de'}}
SEGMENTS = ('importer distributor','homeware brand OEM ODM','kitchen ecommerce retailer','commercial facility janitorial supplier')


def queries(market):
    if market not in MARKETS:
        raise ValueError('Unsupported market')
    country = MARKETS[market]['name']
    return [f'{country} stainless steel trash can {segment} supplier partnership' for segment in SEGMENTS]


class SearchProvider(Protocol):
    def search(self, query: str, market: str) -> list[dict]: ...


class CsvSearchProvider:
    def __init__(self, path):
        with Path(path).open(encoding='utf-8-sig',newline='') as f:
            self.data = list(csv.DictReader(f))
        if any(not r.get('name') or not r.get('website') for r in self.data):
            raise ValueError('CSV requires name and website columns')

    def search(self, query, market):
        return [r for r in self.data if r.get('market',market)==market]


class BraveSearchProvider:
    """Optional keyed service. Replace this adapter without changing evidence/review policy."""
    def __init__(self, api_key=None):
        self.api_key = api_key or os.environ.get('BRAVE_SEARCH_API_KEY')
        if not self.api_key:
            raise ValueError('Set BRAVE_SEARCH_API_KEY, or use --seeds CSV without an API key')

    def search(self, query, market):
        params = urlencode({'q':query, 'country':market, 'search_lang':MARKETS[market]['language'], 'count':10})
        req = urllib.request.Request('https://api.search.brave.com/res/v1/web/search?'+params,
                headers={'X-Subscription-Token':self.api_key,'Accept':'application/json'})
        # No key is logged or included in source URLs.
        with urllib.request.urlopen(req, timeout=20) as response:
            payload = response.read(2_000_001)
        if len(payload)>2_000_000:
            raise ValueError('Search response too large')
        data = json.loads(payload)
        hits=[]
        for r in data.get('web',{}).get('results',[]):
            url=canonical_url(r['url'])
            host=urlsplit(url).hostname
            if host in ('www.linkedin.com','linkedin.com','www.facebook.com','www.amazon.com','www.youtube.com'):
                continue
            hits.append({'name':r.get('title',host),'website':url,'market':market,'customer_type':'unclassified',
                         'discovery_url':url,'query':query})
        return hits


def discover(db, provider, market, limit=20):
    if not 1<=limit<=100:
        raise ValueError('Candidate limit must be 1..100')
    seen, ids, errors = set(), [], []
    for query in queries(market):
        try:
            hits = provider.search(query,market)
        except (ValueError,OSError) as exc:
            errors.append(f'{query}: {type(exc).__name__}')
            continue
        for hit in hits:
            try:
                domain=company_key(hit['website'])
                if domain in seen:
                    continue
                cid=db.upsert_company(hit['name'],hit['website'],market,hit.get('customer_type',''))
                with db.tx():
                    db.audit('system','discovery',cid,{'query':query,'url':hit['website'],'provider':type(provider).__name__})
                seen.add(domain)
                ids.append(cid)
            except (ValueError,KeyError) as exc:
                errors.append(f'Invalid search hit: {type(exc).__name__}')
            if len(ids)>=limit:
                break
        if len(ids)>=limit:
            break
    return {'target':20,'candidate_limit':limit,'unique_candidates':len(ids),'company_ids':ids,'errors':errors,
            'note':'Candidate domains are unverified; 20 qualified customers is a review target, not a search result count.'}
