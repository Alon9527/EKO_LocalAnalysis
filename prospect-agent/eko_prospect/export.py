"""Auditable, formula-safe CSV and qualification progress."""
import csv
import json
from pathlib import Path
from .policy import score, usable


def portfolio(db, as_of, market='US'):
    results=[]
    groups=set()
    for c in db.rows('SELECT * FROM companies WHERE market=? ORDER BY id',(market,)):
        s=score(db,c['id'],as_of)
        results.append({'company':c,'assessment':s})
        if s['qualified']:
            groups.add(c['group_key'])
    results.sort(key=lambda r:(-r['assessment']['score'],r['company']['name']))
    return {'target':20,'market':market,'candidates':len(results),'qualified':len(groups),
            'remaining':max(0,20-len(groups)),'results':results}


def csv_safe(value):
    text=str(value) if value is not None else ''
    # Excel/Sheets may ignore leading whitespace before formula prefixes.
    return "'"+text if text.lstrip().startswith(('=','+','-','@')) or text.startswith(('\t','\r','\n')) else text


def export_csv(db,path,as_of,market='US'):
    path=Path(path)
    path.parent.mkdir(parents=True,exist_ok=True)
    fields=['company_id','name','website','market','customer_type','group_key','relationship','review_status',
            'qualified','score','raw_total','rubric_version','as_of','window_start','verified_procurement',
            'score_breakdown','public_emails','linkedin','contacts_json','missing','blockers','cooperation_angle_recommendation',
            'reviewer','review_note','evidence_json','sources_json']
    with path.open('w',encoding='utf-8-sig',newline='') as f:
        writer=csv.DictWriter(f,fieldnames=fields)
        writer.writeheader()
        for item in portfolio(db,as_of,market)['results']:
            c,s=item['company'],item['assessment']
            evidence=db.evidence(c['id'])
            current=[e for e in evidence if usable(e,as_of)]
            contacts=db.contacts(c['id'])
            sources=db.rows('SELECT id,url,sha256,retrieved_at,status,method,captured_by,first_party,error,active FROM sources WHERE company_id=?',(c['id'],))
            row={k:c.get(k,'') for k in fields}
            row.update({k:s[k] for k in ('qualified','score','raw_total','rubric_version','as_of','window_start','verified_procurement')})
            row.update(company_id=c['id'],score_breakdown=json.dumps(s['breakdown'],ensure_ascii=False),
                       public_emails='; '.join(sorted({e['value'] for e in current if e['kind']=='email'})),
                       linkedin='; '.join(sorted({e['value'] for e in current if e['kind']=='linkedin'})),
                       contacts_json=json.dumps(contacts,ensure_ascii=False),missing='; '.join(s['missing']),
                       blockers='; '.join(s['blockers']),cooperation_angle_recommendation=c['angle'],
                       evidence_json=json.dumps(evidence,ensure_ascii=False),sources_json=json.dumps(sources,ensure_ascii=False))
            writer.writerow({k:csv_safe(row[k]) for k in fields})
    return str(path)
