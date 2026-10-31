"""CLI entrypoint, reusable from Codex, shell scripts or a future orchestrator."""
import argparse
from datetime import date
import json
from pathlib import Path
import sqlite3
import sys
from .store import Store, KINDS
from .agent import run
from .policy import score
from .pipeline import collect, import_bundle
from .providers import CsvSearchProvider, BraveSearchProvider, discover, queries
from .fetch import Fetcher, TrustedProxyTransport
from .export import export_csv, portfolio
from .outreach import prepare_draft, record_sent
from .web import make_server


def parser():
    p=argparse.ArgumentParser(prog='eko-prospect',description='EKO public prospect research: evidence first, human approval, no automated email')
    p.add_argument('--db',default='workspace/prospects.sqlite')
    p.add_argument('--as-of',type=date.fromisoformat,default=None,help='YYYY-MM-DD for reproducible assessment; drafts always use today')
    sub=p.add_subparsers(dest='command',required=True)
    sub.add_parser('init')
    a=sub.add_parser('add-company');a.add_argument('--name',required=True);a.add_argument('--website',required=True);a.add_argument('--market',choices=('US','CA','GB','DE'),default='US');a.add_argument('--type',default='')
    a=sub.add_parser('discover');g=a.add_mutually_exclusive_group(required=True);g.add_argument('--seeds',type=Path);g.add_argument('--brave',action='store_true');a.add_argument('--market',choices=('US','CA','GB','DE'),default='US');a.add_argument('--limit',type=int,default=20)
    a=sub.add_parser('run');g=a.add_mutually_exclusive_group(required=True);g.add_argument('--seeds',type=Path);g.add_argument('--brave',action='store_true');a.add_argument('--market',choices=('US','CA','GB','DE'),default='US');a.add_argument('--limit',type=int,default=20);a.add_argument('--max-pages',type=int,default=5);a.add_argument('--trusted-proxy',action='store_true');a.add_argument('--out',type=Path,default=Path('workspace/prospects.csv'))
    a=sub.add_parser('queries');a.add_argument('--market',choices=('US','CA','GB','DE'),default='US')
    a=sub.add_parser('collect');a.add_argument('--company',type=int,required=True);a.add_argument('--url',action='append',required=True);a.add_argument('--max-pages',type=int,default=5);a.add_argument('--trusted-proxy',action='store_true',help='Use existing controlled HTTP(S)_PROXY; gateway must enforce public destination policy')
    a=sub.add_parser('import');a.add_argument('file',type=Path)
    a=sub.add_parser('add-claim');a.add_argument('--company',type=int,required=True);a.add_argument('--source',type=int,required=True);a.add_argument('--kind',choices=sorted(KINDS),required=True);a.add_argument('--quote',required=True);a.add_argument('--value',default='');a.add_argument('--event-date');a.add_argument('--date-quote',default='')
    a=sub.add_parser('review-claim');a.add_argument('--independent-source',action='store_true');a.add_argument('--claim',type=int,required=True);a.add_argument('--status',choices=('verified','rejected','pending'),required=True);a.add_argument('--actor',required=True);a.add_argument('--note',required=True)
    a=sub.add_parser('review-company');a.add_argument('--expected-revision',type=int);a.add_argument('--company',type=int,required=True);a.add_argument('--status',choices=('approved','hold','rejected'),required=True);a.add_argument('--actor',required=True);a.add_argument('--note',required=True);a.add_argument('--relationship',choices=('unknown','new','existing','do_not_contact'),default='unknown');a.add_argument('--group-key');a.add_argument('--angle',default='')
    a=sub.add_parser('list');a.add_argument('--market',choices=('US','CA','GB','DE'),default='US')
    a=sub.add_parser('show');a.add_argument('--company',type=int,required=True)
    a=sub.add_parser('export');a.add_argument('--out',type=Path,default=Path('workspace/prospects.csv'));a.add_argument('--market',choices=('US','CA','GB','DE'),default='US')
    a=sub.add_parser('serve');a.add_argument('--port',type=int,default=8765)
    a=sub.add_parser('draft');a.add_argument('--company',type=int,required=True);a.add_argument('--email',required=True);a.add_argument('--subject',required=True);a.add_argument('--body-file',type=Path,required=True);a.add_argument('--actor',required=True);a.add_argument('--confirm-recipient',required=True);a.add_argument('--out-dir',type=Path,default=Path('workspace/drafts'))
    a=sub.add_parser('sent');a.add_argument('--draft',type=int,required=True);a.add_argument('--actor',required=True);a.add_argument('--note',required=True)
    a=sub.add_parser('suppress');a.add_argument('--key',required=True);a.add_argument('--actor',required=True);a.add_argument('--reason',required=True)
    sub.add_parser('audit')
    return p


def main(argv=None):
    args=parser().parse_args(argv)
    as_of=args.as_of or date.today()
    db=Store(args.db)
    try:
        cmd=args.command
        result={}
        if cmd=='init': result={'db':args.db,'schema_version':2}
        elif cmd=='add-company': result={'company_id':db.upsert_company(args.name,args.website,args.market,args.type)}
        elif cmd=='queries': result={'queries':queries(args.market)}
        elif cmd=='discover': result=discover(db,CsvSearchProvider(args.seeds) if args.seeds else BraveSearchProvider(),args.market,args.limit)
        elif cmd=='run': result=run(db,CsvSearchProvider(args.seeds) if args.seeds else BraveSearchProvider(),market=args.market,limit=args.limit,max_pages=args.max_pages,fetcher=Fetcher(TrustedProxyTransport() if args.trusted_proxy else None),as_of=as_of,out=args.out)
        elif cmd=='collect': result=collect(db,args.company,args.url,Fetcher(TrustedProxyTransport() if args.trusted_proxy else None),max_pages=args.max_pages)
        elif cmd=='import': result=import_bundle(db,json.loads(args.file.read_text(encoding='utf-8-sig')))
        elif cmd=='add-claim': result={'claim_id':db.add_claim(args.company,args.source,args.kind,args.quote,value=args.value,event_date=args.event_date,date_quote=args.date_quote)}
        elif cmd=='review-claim':
            db.review_claim(args.claim,args.status,args.actor,args.note,independent=args.independent_source);result={'claim_id':args.claim,'status':args.status}
        elif cmd=='review-company':
            db.review_company(args.company,args.status,args.actor,args.note,relationship=args.relationship,group_key=args.group_key,angle=args.angle,as_of=as_of,expected_revision=args.expected_revision);result={'company_id':args.company,'status':args.status}
        elif cmd=='list': result=portfolio(db,as_of,args.market)
        elif cmd=='show': result={'company':db.company(args.company),'assessment':score(db,args.company,as_of),'evidence':db.evidence(args.company),'contacts':db.contacts(args.company),'sources':db.rows('SELECT id,url,retrieved_at,status,error,method,active FROM sources WHERE company_id=?',(args.company,))}
        elif cmd=='export': result={'csv':export_csv(db,args.out,as_of,args.market)}
        elif cmd=='serve':
            server=make_server(args.db,args.port,args.as_of)
            print(f'Open http://127.0.0.1:{server.server_port} — Ctrl+C to stop.',flush=True)
            try: server.serve_forever()
            except KeyboardInterrupt: pass
            finally: server.server_close()
            return 0
        elif cmd=='draft':
            if args.as_of and args.as_of!=date.today():
                raise ValueError('Drafts cannot use a historical assessment date; omit --as-of')
            args.out_dir.mkdir(parents=True,exist_ok=True)
            result=prepare_draft(db,args.company,args.email,args.subject,args.body_file.read_text(encoding='utf-8'),args.actor,args.confirm_recipient,date.today())
            path=args.out_dir/f'draft-{result["id"]}.eml'
            # A failed file write leaves the durable reservation in place, avoiding duplicate contacts.
            path.write_text(result.pop('eml'),encoding='utf-8',newline='')
            result['file']=str(path)
        elif cmd=='sent': record_sent(db,args.draft,args.actor,args.note);result={'draft_id':args.draft,'status':'sent_recorded_only'}
        elif cmd=='suppress': db.suppress(args.key,args.reason,args.actor);result={'suppressed':args.key}
        elif cmd=='audit': result={'audit':db.rows('SELECT * FROM audit ORDER BY id')}
        print(json.dumps(result,ensure_ascii=False,indent=2))
        # Mixed acquisition failures stay visible in JSON and return nonzero for automation.
        if cmd=='collect' and (result['failed'] or result['errors']): return 2
        if cmd=='discover' and result['errors']: return 2
        if cmd=='run' and (result['discovery']['errors'] or any(c['failed'] or c['errors'] for c in result['collections'])): return 2
        return 0
    except (ValueError,KeyError,OSError,sqlite3.Error) as exc:
        print(f'Error: {exc}',file=sys.stderr)
        return 2
    finally:
        db.close()
