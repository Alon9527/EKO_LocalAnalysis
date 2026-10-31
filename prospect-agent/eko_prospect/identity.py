"""Canonical identities without guessing registrable domains or email aliases."""
import calendar
import re
from datetime import date, datetime, timezone
from urllib.parse import parse_qsl, urlencode, urlsplit, urlunsplit


def canonical_url(value: str) -> str:
    p = urlsplit(value.strip())
    if p.scheme not in ('http', 'https') or not p.hostname or p.username or p.password:
        raise ValueError('Expected an absolute public HTTP(S) URL without credentials')
    host = p.hostname.encode('idna').decode().lower().rstrip('.')
    if ':' in host:
        host = '[' + host + ']'
    if p.port and p.port != (443 if p.scheme == 'https' else 80):
        host += f':{p.port}'
    query = [(k, v) for k, v in parse_qsl(p.query, keep_blank_values=True)
             if not k.lower().startswith('utm_') and k.lower() not in ('fbclid', 'gclid', 'msockid')]
    return urlunsplit((p.scheme.lower(), host, p.path or '/', urlencode(query), ''))


def company_key(url: str) -> str:
    host = urlsplit(canonical_url(url)).hostname
    return host.removeprefix('www.')


def email_key(value: str) -> str:
    email = value.strip().lower()
    if len(email) > 254 or not re.fullmatch(r"[a-z0-9.!#$%&'*+/=?^_`{|}~-]+@[a-z0-9](?:[a-z0-9.-]*[a-z0-9])?\.[a-z]{2,}", email):
        raise ValueError('Invalid public email address')
    return email


def linkedin_key(value: str) -> str:
    url = canonical_url(value)
    p = urlsplit(url)
    if p.hostname not in ('linkedin.com', 'www.linkedin.com') or not re.fullmatch(r'/(?:company|in)/[^/]+/?', p.path):
        raise ValueError('Expected a LinkedIn company or personal profile URL')
    return 'https://www.linkedin.com' + p.path.rstrip('/').lower()


def months_before(day: date, months: int = 6) -> date:
    index = day.year * 12 + day.month - 1 - months
    year, month = divmod(index, 12)
    month += 1
    return date(year, month, min(day.day, calendar.monthrange(year, month)[1]))


def norm(text: str) -> str:
    return ' '.join(text.split())


def utc_now() -> str:
    return datetime.now(timezone.utc).isoformat(timespec='seconds')


def timestamp(value: str) -> datetime:
    result = datetime.fromisoformat(value.replace('Z', '+00:00'))
    if result.tzinfo is None:
        raise ValueError('Timestamp must include timezone')
    return result.astimezone(timezone.utc)


def parse_date(text: str) -> date | None:
    text = text.strip()
    for fmt in ('%Y-%m-%d', '%B %d, %Y', '%b %d, %Y', '%d %B %Y', '%d %b %Y'):
        try:
            return datetime.strptime(text, fmt).date()
        except ValueError:
            pass
    return None


def email_tokens(text: str):
    """Linear tokenization, then bounded full matches; never search a long token repeatedly."""
    for token in re.finditer(r"[^\s<>\"(),;:]+", text):
        value = token.group().rstrip('.!?')
        if '@' not in value or len(value) > 254:
            continue
        try:
            yield email_key(value), token.start(), token.start() + len(value)
        except ValueError:
            continue


def public_emails(text: str) -> set[str]:
    return {value for value, _, _ in email_tokens(text)}


def linkedin_urls(text: str) -> set[str]:
    result = set()
    for match in re.finditer(r"https?://[^\s<>\"')\]]+", text):
        try:
            result.add(linkedin_key(match.group().rstrip('.,;:!?')))
        except ValueError:
            continue
    return result
