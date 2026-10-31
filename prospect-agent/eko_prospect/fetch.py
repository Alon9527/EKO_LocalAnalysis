"""Bounded public-page fetcher, honoring robots and pinning validated DNS addresses."""
from dataclasses import dataclass
from html.parser import HTMLParser
import http.client
import ipaddress
import re
import socket
import ssl
import time
from urllib.parse import urljoin, urlsplit
from urllib.robotparser import RobotFileParser
from .identity import canonical_url, utc_now

USER_AGENT = 'EKOProspectResearch/0.1'
MAX_BYTES = 2_000_000


class UnsafeURL(ValueError):
    pass


def check_url(url):
    url = canonical_url(url)
    p = urlsplit(url)
    if p.port not in (None, 80, 443):
        raise UnsafeURL('Only public HTTP(S) standard ports allowed')
    host = p.hostname
    if host == 'localhost' or host.endswith(('.localhost', '.local', '.internal')) or '.' not in host and ':' not in host:
        raise UnsafeURL('Local hostnames are not allowed')
    try:
        ip = ipaddress.ip_address(host)
    except ValueError:
        return url
    if not ip.is_global:
        raise UnsafeURL('Non-public IP address denied')
    return url


def public_addresses(url):
    p = urlsplit(check_url(url))
    answers = socket.getaddrinfo(p.hostname, p.port or (443 if p.scheme=='https' else 80), type=socket.SOCK_STREAM)
    addresses = list(dict.fromkeys(a[4][0] for a in answers))
    if not addresses or any(not ipaddress.ip_address(ip).is_global for ip in addresses):
        raise UnsafeURL('DNS resolves to non-public or empty address set')
    return addresses


@dataclass
class Response:
    status: int
    headers: dict
    body: bytes


class PublicTransport:
    def get(self, url):
        p = urlsplit(check_url(url))
        addresses = public_addresses(url)
        # Connect to the validated IP, not a second hostname lookup (DNS rebinding).
        port = p.port or (443 if p.scheme == 'https' else 80)
        last_error = None
        for ip in addresses:
            conn = http.client.HTTPConnection(p.hostname, port, timeout=15)
            try:
                sock = socket.create_connection((ip, port), timeout=15)
                try:
                    if p.scheme == 'https':
                        sock = ssl.create_default_context().wrap_socket(sock, server_hostname=p.hostname)
                    conn.sock = sock
                except Exception:
                    sock.close()
                    raise
                conn.request('GET', p.path + ('?' + p.query if p.query else ''), headers={
                    'Host': p.netloc, 'User-Agent': USER_AGENT, 'Accept': 'text/html,text/plain', 'Accept-Encoding': 'identity'})
                res = conn.getresponse()
                body = res.read(MAX_BYTES + 1)
                if len(body) > MAX_BYTES:
                    raise ValueError('Response exceeds 2 MB limit')
                return Response(res.status, {k.lower():v for k,v in res.getheaders()}, body)
            except (OSError, http.client.HTTPException) as exc:
                last_error = exc
            finally:
                conn.close()
        raise OSError(str(last_error or 'No reachable public address'))


class TextParser(HTMLParser):
    def __init__(self):
        super().__init__(convert_charrefs=True)
        self.parts = []
        self.hidden = []
        self.links = []

    def handle_starttag(self, tag, attrs):
        attrs = dict(attrs)
        if tag in ('script','style','noscript','template'):
            self.hidden.append(tag)
        if self.hidden:
            return
        if tag in ('p','div','h1','h2','h3','h4','li','br','section','article'):
            self.parts.append('\n')
        if tag == 'a':
            href = attrs.get('href','')
            self.links.append(href)
            if href.startswith('mailto:'):
                self.parts.append(' ' + href[7:].split('?')[0] + ' ')
            elif 'linkedin.com/' in href:
                self.parts.append(' ' + href + ' ')

    def handle_endtag(self, tag):
        if self.hidden:
            if tag == self.hidden[-1]:
                self.hidden.pop()
        elif tag in ('p','div','li','h1','h2','h3','h4','article','section'):
            self.parts.append('\n')

    def handle_data(self, data):
        if not self.hidden:
            self.parts.append(data)


def html_text(html):
    parser = TextParser()
    parser.feed(html)
    return '\n'.join(' '.join(line.split()) for line in ''.join(parser.parts).splitlines() if line.strip())


class Fetcher:
    def __init__(self, transport=None, delay=1.0, retries=1, sleeper=time.sleep):
        self.transport = transport or PublicTransport()
        self.delay = max(0, delay)
        self.retries = min(2, max(0,retries))
        self.sleep = sleeper
        self.last = {}
        self.robots = {}

    def request(self, url):
        origin = urlsplit(url).netloc
        for attempt in range(self.retries + 1):
            elapsed = time.monotonic() - self.last.get(origin, 0)
            if elapsed < self.delay:
                self.sleep(self.delay - elapsed)
            self.last[origin] = time.monotonic()
            response = self.transport.get(url)
            if response.status not in (429,500,502,503,504) or attempt == self.retries:
                return response
            self.sleep(min(5, 2 ** attempt))
        raise RuntimeError('Unreachable retry state')

    def allowed(self, url):
        p = urlsplit(url)
        origin = p.scheme + '://' + p.netloc
        if origin not in self.robots:
            r = self.request(origin + '/robots.txt')
            if r.status == 404:
                self.robots[origin] = None
            elif r.status != 200:
                raise UnsafeURL(f'robots.txt unavailable (HTTP {r.status}); manual review required')
            else:
                rp = RobotFileParser()
                rp.parse(r.body.decode('utf-8', errors='replace').splitlines())
                self.robots[origin] = rp
        rp = self.robots[origin]
        if rp:
            crawl_delay = rp.crawl_delay(USER_AGENT) or rp.crawl_delay('*') or 0
            if crawl_delay > 60:
                raise UnsafeURL('robots crawl delay exceeds interactive budget')
            self.delay = max(self.delay, crawl_delay)
        return rp is None or rp.can_fetch(USER_AGENT, url)

    def fetch(self, url):
        requested = canonical_url(url)
        result = {'url': requested, 'text': '', 'retrieved_at': utc_now(), 'status': 'error', 'method':'http', 'error':''}
        try:
            current = check_url(requested)
            for _ in range(4):
                host = urlsplit(current).hostname
                if host == 'linkedin.com' or host.endswith('.linkedin.com'):
                    raise UnsafeURL('LinkedIn requires manual public-source review')
                if not self.allowed(current):
                    raise UnsafeURL('robots.txt denies this path')
                r = self.request(current)
                if r.status in (301,302,303,307,308):
                    current = check_url(urljoin(current, r.headers.get('location','')))
                    continue
                if r.status != 200:
                    raise ValueError(f'HTTP {r.status}; not collected')
                if len(r.body) > MAX_BYTES:
                    raise ValueError('Response exceeds 2 MB limit')
                mime = r.headers.get('content-type','').split(';')[0].strip().lower()
                if mime not in ('text/html','text/plain','application/xhtml+xml'):
                    raise ValueError(f'Unsupported content type {mime}; import a public excerpt manually')
                match = re.search(r'charset=([\w-]+)',r.headers.get('content-type',''))
                encoding = match.group(1) if match else 'utf-8'
                text = r.body.decode(encoding, errors='replace')
                text = html_text(text) if mime != 'text/plain' else text
                if len(text.strip()) < 10 or re.search(r'^(Just a moment|Access Denied|Verify you are human)',text, re.I):
                    raise ValueError('Empty or challenge page; manual review required')
                # Preserve requested URL separately in caller audit; source identity uses actual URL.
                links=[]
                if mime != 'text/plain':
                    parser=TextParser()
                    parser.feed(r.body.decode(encoding,errors='replace'))
                    for href in parser.links:
                        try:
                            link=check_url(urljoin(current,href))
                            if urlsplit(link).hostname==urlsplit(current).hostname:
                                links.append(link)
                        except ValueError:
                            continue
                result.update(url=current, text=text, status='ok', retrieved_at=utc_now(),links=list(dict.fromkeys(links)))
                return result
            raise UnsafeURL('Redirect limit exceeded')
        except UnsafeURL as exc:
            result.update(status='blocked', error=str(exc))
        except (OSError,ValueError,LookupError,http.client.HTTPException) as exc:
            result.update(error=f'{type(exc).__name__}: {exc}')
        return result


class TrustedProxyTransport:
    """Explicit opt-in for a controlled egress proxy that enforces public DNS/IP policy.

    DNS is performed by the proxy. Only use with a trusted gateway that blocks
    private/internal destinations. Default PublicTransport provides local IP pinning.
    """
    def __init__(self, proxies=None):
        import urllib.request
        proxies=urllib.request.getproxies() if proxies is None else proxies
        proxies={k:v for k,v in proxies.items() if k in ('http','https')}
        if not proxies:
            raise ValueError('Trusted proxy mode requires HTTP_PROXY or HTTPS_PROXY')
        class NoRedirect(urllib.request.HTTPRedirectHandler):
            def redirect_request(self,req,fp,code,msg,headers,newurl):
                return None
        self.proxies = proxies
        class ForcedProxyHandler(urllib.request.ProxyHandler):
            def proxy_open(self, req, proxy, type):
                # Explicit trusted-gateway mode must never obey NO_PROXY or fall back to direct.
                from urllib.parse import unquote
                import base64
                p=urlsplit(proxy)
                if p.scheme not in ('http','https') or not p.hostname:
                    raise ValueError('Proxy must be an absolute HTTP(S) URL')
                if p.username is not None:
                    credentials=unquote(p.username)+':'+unquote(p.password or '')
                    req.add_unredirected_header('Proxy-Authorization','Basic '+base64.b64encode(credentials.encode()).decode('ascii'))
                host=p.hostname if ':' not in p.hostname else '['+p.hostname+']'
                if p.port:
                    host += ':'+str(p.port)
                req.set_proxy(host,p.scheme)
                return None
        self.opener=urllib.request.build_opener(ForcedProxyHandler(proxies),NoRedirect())

    def get(self,url):
        import urllib.request
        import urllib.error
        url=check_url(url)
        if urlsplit(url).scheme not in self.proxies:
            raise ValueError('No trusted proxy configured for requested URL scheme; refusing direct fallback')
        req=urllib.request.Request(url,headers={'User-Agent':USER_AGENT,'Accept':'text/html,text/plain','Accept-Encoding':'identity'})
        try:
            response=self.opener.open(req,timeout=20)
        except urllib.error.HTTPError as exc:
            response=exc
        with response:
            body=response.read(MAX_BYTES+1)
            if len(body)>MAX_BYTES:
                raise ValueError('Response exceeds 2 MB limit')
            return Response(response.code,{k.lower():v for k,v in response.headers.items()},body)
