"""Loopback-only review console. State changes require a per-process CSRF token."""
from datetime import date
from html import escape
from http.server import HTTPServer, BaseHTTPRequestHandler
import json
from pathlib import Path
import secrets
from urllib.parse import parse_qs, urlsplit
from .store import Store, KINDS
from .policy import score
from .export import portfolio, export_csv

STYLE='''body{margin:0;background:#f4f6f5;color:#18322b;font:15px/1.6 system-ui,sans-serif}main{max-width:1180px;margin:auto;padding:32px}h1{font-size:30px;margin:0}a{color:#087359;overflow-wrap:anywhere}header{border-bottom:1px solid #d7e2dc;padding-bottom:20px;margin-bottom:24px}.muted{color:#67786e}section,.card{background:white;border:1px solid #dde6e0;border-radius:12px;padding:20px;margin:16px 0}.stats{display:flex;gap:16px}.stats div{flex:1;background:#e7eee9;padding:16px;border-radius:10px}.stats strong{font-size:26px;display:block}table{width:100%;border-collapse:collapse}td,th{border-bottom:1px solid #e5eae7;padding:10px;text-align:left;vertical-align:top}input,select,textarea{border:1px solid #c4d3c9;border-radius:6px;padding:9px;font:inherit;max-width:100%;box-sizing:border-box}textarea{width:100%;min-height:70px}button{background:#146c50;color:white;border:0;border-radius:6px;padding:9px 14px;cursor:pointer}form{margin-top:12px;display:flex;gap:10px;flex-wrap:wrap;align-items:center}form label{display:block}small{font-size:12px}pre{white-space:pre-wrap;overflow-wrap:anywhere;background:#f3f5f3;padding:12px}blockquote{border-left:3px solid #80a592;margin:12px 0;padding-left:14px}.tag{background:#e8eee9;border-radius:4px;padding:3px 8px}.warning{color:#945b14}summary{cursor:pointer} @media(max-width:700px){main{padding:14px}.stats{flex-wrap:wrap}table{display:block;overflow:auto}}'''


def h(v):
    return escape(str(v) if v is not None else '',quote=True)


def page(body):
    return '<!doctype html><html lang="zh-CN"><meta charset="utf-8"><meta name="viewport" content="width=device-width, initial-scale=1"><title>EKO 客户开发复核台</title><style>'+STYLE+'</style><main><header><h1>EKO 客户开发复核台</h1><div class="muted">公开来源 · 逐项核验 · 人工决定 · 本地保存</div></header>'+body+'</main></html>'


def make_server(db_path,port=8765,as_of=None):
    token=secrets.token_urlsafe(32)
    def day():
        return as_of or date.today()
    def hidden():
        return f'<input type="hidden" name="csrf" value="{token}">'
    class Handler(BaseHTTPRequestHandler):
        def log_message(self,*args):
            pass

        def send(self,status,body,content_type='text/html; charset=utf-8'):
            data=body.encode() if isinstance(body,str) else body
            self.send_response(status)
            self.send_header('Content-Type',content_type)
            self.send_header('Content-Length',str(len(data)))
            self.send_header('Cache-Control','no-store')
            self.send_header('X-Content-Type-Options','nosniff')
            self.send_header('Content-Security-Policy',"default-src 'none'; style-src 'unsafe-inline'; form-action 'self'; frame-ancestors 'none'; base-uri 'none'")
            self.end_headers()
            self.wfile.write(data)

        def host_ok(self):
            expected={f'127.0.0.1:{self.server.server_port}',f'localhost:{self.server.server_port}'}
            return self.headers.get('Host','') in expected

        def do_GET(self):
            if not self.host_ok():
                self.send(403,'Forbidden');return
            db=Store(db_path)
            try:
                url=urlsplit(self.path)
                params=parse_qs(url.query)
                if url.path=='/':
                    market=params.get('market',['US'])[0]
                    p=portfolio(db,day(),market)
                    body=f'<p>评估日期：{day()} · <a href="/?market=US">美国</a> / <a href="/?market=CA">加拿大</a> / <a href="/?market=GB">英国</a> / <a href="/?market=DE">德国</a></p>'
                    body+=f'<div class="stats"><div>候选企业<strong>{p["candidates"]}</strong></div><div>经批准的独立集团<strong>{p["qualified"]} / 20</strong></div><div>还需核验<strong>{p["remaining"]}</strong></div></div>'
                    body+='<p class="muted">1 分是无有效证据时的显示下限。分数代表开发优先级，不代表采购概率。所有缺失字段保留为空。</p>'
                    body+=f'<a href="/export?market={h(market)}">下载 CSV（含来源、引文、缺失项与审核记录）</a><section><table><tr><th>企业</th><th>评分</th><th>人审</th><th>待补充</th></tr>'
                    for r in p['results']:
                        c,s=r['company'],r['assessment']
                        body+=f'<tr><td><a href="/company?id={c["id"]}">{h(c["name"])}</a><br><small>{h(c["domain"])}</small></td><td>{s["score"]}/10</td><td>{h(c["review_status"])}</td><td>{h(", ".join(s["blockers"]))}</td></tr>'
                    body+='</table></section><p class="muted">新建候选、搜索与采集请使用 README 中的命令。审核状态只能由你确认。邮件草稿需要 CLI 逐封确认，程序无发信功能。</p>'
                    self.send(200,page(body))
                elif url.path=='/company':
                    cid=int(params['id'][0]);c=db.company(cid);s=score(db,cid,day())
                    body=f'<a href="/">← 返回客户列表</a><h2>{h(c["name"])} <span class="tag">{s["score"]}/10</span></h2><a href="{h(c["website"])}" target="_blank" rel="noreferrer">{h(c["website"])}</a>'
                    body+='<p>评分分解：'+h(json.dumps(s['breakdown'],ensure_ascii=False))+'</p><p class="warning">批准阻碍：'+h(', '.join(s['blockers']) or '无；请确认 EKO 合作切入点后批准')+'</p>'
                    body+='<p>事件核验时确认日期属于该事件、是否与垃圾桶品类有关、是否仅表示渠道扩张。不能用抓取日期代替事件日期。</p>'
                    for e in db.evidence(cid):
                        body+=f'<section><b>#{e["id"]} · {h(e["kind"])} · {h(e["status"])}</b> <small>来源 #{e["source_id"]} / {"官方域名" if e["first_party"] else "外部域名"} / {"有效版本" if e["active"] else "已被新版本替代"}</small><blockquote>{h(e["quote"])}</blockquote><div>值：{h(e["value"] or "未填")} · 事件日期：{h(e["event_date"] or "缺失/不适用")}</div><div>日期原文：{h(e["date_quote"] or "—")}</div><a href="{h(e["url"])}" target="_blank" rel="noreferrer">打开来源</a> · 抓取 {h(e["retrieved_at"])} · {h(e["method"])}<br><small>SHA256 {h(e["sha256"])}</small>'
                        body+=f'<form method="post" action="/review-claim">{hidden()}<input type="hidden" name="id" value="{e["id"]}"><select name="status"><option value="verified">核实属实</option><option value="rejected">证据不支持</option><option value="pending">待补充</option></select><input name="actor" placeholder="审核人" required><input name="note" placeholder="核验依据/拒绝原因" required><label><input type="checkbox" name="independent" value="yes">独立外部采购来源（仅采购证据适用）</label><button>保存此项审核</button></form></section>'
                    sources=db.rows('SELECT * FROM sources WHERE company_id=? ORDER BY id',(cid,))
                    body+='<section><h3>来源与采集失败记录</h3>'
                    for source in sources:
                        body+=f'<details><summary>#{source["id"]} {h(source["status"])} · {h(source["url"])}</summary><p>{h(source["error"])}</p><pre>{h(source["text"][:50000])}</pre></details>'
                    body+='</section><section><h3>补充证据（必须来自已留存正文）</h3>'
                    options=''.join(f'<option value="{h(k)}">{h(k)}</option>' for k in sorted(KINDS))
                    body+=f'<form method="post" action="/add-claim">{hidden()}<input type="hidden" name="company_id" value="{cid}"><input type="number" name="source_id" placeholder="来源 ID" required><select name="kind">{options}</select><input name="value" placeholder="值，例如 US / 邮箱 / 事件标识"><input name="event_date" placeholder="事件日 YYYY-MM-DD"><input name="date_quote" placeholder="原文日期（事件必填）"><textarea name="quote" placeholder="原文连续短摘录，最多25词" required></textarea><button>加入待核验证据</button></form></section>'
                    body+=f'<section><h3>客户最终复核</h3><p class="muted">new = 已查内部 CRM 确认新客户；existing = 既有渠道扩品。group_key 用于关联同集团，防止重复开发。</p><form method="post" action="/review-company">{hidden()}<input type="hidden" name="id" value="{cid}"><input type="hidden" name="expected_revision" value="{c["revision"]}"><select name="status"><option value="hold">暂缓/待补充</option><option value="approved">批准进入开发</option><option value="rejected">不适合</option></select><select name="relationship">'
                    for rel in ('unknown','new','existing','do_not_contact'):
                        body+=f'<option value="{rel}" {"selected" if c["relationship"]==rel else ""}>{rel}</option>'
                    body+=f'</select><input name="group_key" value="{h(c["group_key"])}" required><input name="actor" placeholder="审核人" required><input name="note" placeholder="CRM核查/审批理由" required><textarea name="angle" placeholder="EKO合作切入点（建议，非事实）">{h(c["angle"])}</textarea><button>保存客户审核</button></form></section>'
                    self.send(200,page(body))
                elif url.path=='/export':
                    market=params.get('market',['US'])[0]
                    import tempfile
                    with tempfile.TemporaryDirectory() as tmp:
                        target=Path(tmp)/'prospects.csv'
                        export_csv(db,target,day(),market)
                        data=target.read_bytes()
                    self.send(200,data,'text/csv; charset=utf-8')
                else:
                    self.send(404,'Not found')
            except (ValueError,KeyError) as exc:
                self.send(400,page('<p>'+h(exc)+'</p>'))
            finally:
                db.close()

        def do_POST(self):
            if not self.host_ok():
                self.send(403,'Forbidden');return
            origin=self.headers.get('Origin')
            if origin and origin not in (f'http://127.0.0.1:{self.server.server_port}',f'http://localhost:{self.server.server_port}'):
                self.send(403,'Forbidden');return
            try:
                length=int(self.headers.get('Content-Length','0'))
                if not 0 < length <= 20000:
                    raise ValueError('Request size invalid')
                p={k:v[0] for k,v in parse_qs(self.rfile.read(length).decode(),keep_blank_values=True).items()}
            except (ValueError,UnicodeError):
                self.send(400,'Bad request');return
            if not secrets.compare_digest(p.get('csrf',''),token):
                self.send(403,'CSRF validation failed');return
            db=Store(db_path)
            try:
                if self.path=='/review-claim':
                    claim_id=int(p['id']);cid=db.claim(claim_id)['company_id']
                    db.review_claim(claim_id,p['status'],p['actor'],p['note'],independent=p.get('independent')=='yes')
                elif self.path=='/review-company':
                    cid=int(p['id'])
                    db.review_company(cid,p['status'],p['actor'],p['note'],relationship=p['relationship'],group_key=p.get('group_key'),angle=p.get('angle',''),as_of=day(),expected_revision=int(p['expected_revision']))
                elif self.path=='/add-claim':
                    cid=int(p['company_id'])
                    db.add_claim(cid,int(p['source_id']),p['kind'],p['quote'],value=p.get('value',''),event_date=p.get('event_date') or None,date_quote=p.get('date_quote',''))
                else:
                    self.send(404,'Not found');return
                self.send_response(303);self.send_header('Location',f'/company?id={cid}');self.send_header('Content-Length','0');self.end_headers()
            except (ValueError,KeyError) as exc:
                self.send(400,page('<p class="warning">'+h(exc)+'</p><a href="/">返回复核台</a>'))
            finally:
                db.close()
    server=HTTPServer(('127.0.0.1',port),Handler)
    return server
