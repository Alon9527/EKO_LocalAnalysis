"""Human-confirmed local drafts and suppression ledger. There is NO sending transport."""
from email.message import EmailMessage
from email.policy import SMTP
import hashlib
import sqlite3
from .identity import email_key, utc_now, public_emails
from .policy import score, usable


def prepare_draft(db,cid,email,subject,body,actor,confirmation,as_of):
    email=email_key(email)
    if confirmation.strip()!=email or not actor.strip():
        raise ValueError('Human confirmation must exactly match the recipient email')
    if not subject.strip() or not body.strip() or '\n' in subject or '\r' in subject:
        raise ValueError('Valid subject and reviewed message body required')
    # BEGIN IMMEDIATE serializes group and recipient reservations across processes.
    if db.conn.in_transaction:
        raise ValueError('Draft preparation requires an independent transaction')
    db.conn.execute('BEGIN IMMEDIATE')
    try:
        c=db.company(cid)
        s=score(db,cid,as_of)
        if not s['qualified']:
            raise ValueError('Company requires a current approved review and qualifying evidence')
        evidence=[e for e in db.evidence(cid) if usable(e,as_of) and e['first_party']]
        if not any(e['kind']=='email' and e['value']==email for e in evidence):
            raise ValueError('Recipient must be a current, verified, explicitly published work email')
        if not any(e['kind']=='supplier_access' and email in public_emails(e['quote']) for e in evidence):
            raise ValueError('Recipient lacks verified supplier-purpose context; customer service/PR is not a buyer')
        if db.rows('SELECT key FROM suppressions WHERE key IN (?,?,?)',(email,c['domain'],c['group_key'])):
            raise ValueError('Recipient or company is suppressed')
        # Joining current companies catches parent-group edits after prior outreach.
        if db.rows('''SELECT o.id FROM outreach o JOIN companies x ON x.id=o.company_id
                WHERE o.company_id=? OR o.email=? OR o.group_key=? OR x.group_key=?''',
                   (cid,email,c['group_key'],c['group_key'])):
            raise ValueError('Company/group/email already has a draft or contact history; review it before any follow-up')
        digest=hashlib.sha256((subject+'\n'+body).encode()).hexdigest()
        cur=db.conn.execute('INSERT INTO outreach(company_id,group_key,email,subject,body,body_sha256,confirmed_by,confirmed_at) VALUES(?,?,?,?,?,?,?,?)',
                            (cid,c['group_key'],email,subject,body,digest,actor,utc_now()))
        draft_id=cur.lastrowid
        db.audit(actor,'outreach_prepared',draft_id,{'company_id':cid,'email':email,'body_sha256':digest,'revision':c['revision']})
        msg=EmailMessage(policy=SMTP)
        msg['To']=email
        msg['Subject']=subject
        msg['X-Unsent']='1'
        msg['X-EKO-Draft-ID']=str(draft_id)
        msg.set_content(body)
        eml=msg.as_string()
        db.conn.commit()
        return {'id':draft_id,'eml':eml,'status':'prepared','note':'Nothing sent. Open in your mail client, review once more, then send manually.'}
    except (ValueError,sqlite3.Error):
        db.conn.rollback()
        raise


def record_sent(db,draft_id,actor,note):
    db.check_review(actor,note)
    with db.tx():
        rows=db.rows('SELECT * FROM outreach WHERE id=?',(draft_id,))
        if not rows:
            raise ValueError('Draft not found')
        db.conn.execute("UPDATE outreach SET status='sent',note=? WHERE id=?",(note,draft_id))
        db.audit(actor,'manual_send_recorded',draft_id,{'note':note})
