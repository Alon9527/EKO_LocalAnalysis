"""One bounded run: discover -> collect -> propose -> assess -> export -> human queue."""
from datetime import date
from .providers import discover
from .pipeline import collect
from .export import export_csv,portfolio
from .fetch import Fetcher


def run(db,provider,*,market='US',limit=20,max_pages=5,fetcher=None,as_of=None,out='workspace/prospects.csv'):
    as_of=as_of or date.today()
    fetcher=fetcher or Fetcher()
    discovery=discover(db,provider,market,limit)
    collections=[]
    for cid in discovery['company_ids']:
        company=db.company(cid)
        collections.append(collect(db,cid,[company['website']],fetcher,max_pages=max_pages,follow_links=True))
    result=portfolio(db,as_of,market)
    export_csv(db,out,as_of,market)
    report={'discovery':discovery,'collections':collections,'portfolio':result,'csv':str(out),
            'next_step':'Open review console; nothing is verified or approved automatically.'}
    with db.tx():
        db.audit('system','run_finished',market,{'candidates':result['candidates'],'qualified':result['qualified'],
                                               'pages_ok':sum(c['ok'] for c in collections),'pages_failed':sum(c['failed'] for c in collections)})
    return report
