"""Replaceable candidate extraction. Never sets a verification status."""
import re
from typing import Protocol
from .identity import norm, email_tokens, linkedin_urls


class Extractor(Protocol):
    def propose(self, text: str, market: str) -> list[dict]: ...


PATTERNS = {
    'category': r'\b(?:trash cans?|waste receptacles?|recycling bins?|rubbish bins?|Abfalleimer)\b',
    'channel': r'\b(?:distributor|distribution|wholesale|retailer|retailers|importer|OEM|ODM)\b',
    'supplier_access': r'\b(?:become a supplier|become a vendor|vendor team|supplier program|supplier application|new vendor|vendor partner submissions)\b',
    'existing_listing': r'\bEKO\b',
}
MARKET_TERMS = {'US':r'\b(?:United States|U\.S\.|USA|US - English)\b',
                'CA':r'\bCanada\b','GB':r'\bUnited Kingdom\b','DE':r'\b(?:Germany|Deutschland)\b'}
EMAIL_RE = r"[a-zA-Z0-9.!#$%&'*+/=?^_`{|}~-]+@[a-zA-Z0-9.-]+\.[A-Za-z]{2,}"


def excerpt(line, start, end):
    # Slice original character spans so adjacent punctuation is preserved exactly.
    words = list(re.finditer(r"\S+", line))
    left = next((i for i,w in enumerate(words) if w.end()>start), 0)
    right = next((i for i,w in enumerate(words) if w.end()>=end), len(words)-1)
    return line[words[max(0,left-8)].start():words[min(len(words)-1,right+8)].end()]


class RuleExtractor:
    def propose(self, text, market):
        proposals, seen = [], set()
        for line in text.splitlines():
            line = norm(line)
            if not line:
                continue
            for kind, pattern in dict(PATTERNS, market=MARKET_TERMS[market]).items():
                m = re.search(pattern, line, re.I)
                if m and kind not in seen:
                    q = excerpt(line,m.start(),m.end())
                    if len(q.split()) <= 25:
                        proposals.append({'kind':kind,'quote':q,'value':market if kind=='market' else ''})
                        seen.add(kind)
            for value,start,end in email_tokens(line):
                if ('email',value) not in seen:
                    proposals.append({'kind':'email','quote':excerpt(line,start,end),'value':value})
                    seen.add(('email',value))
            for value in sorted(linkedin_urls(line)):
                if ('linkedin',value) not in seen:
                    # Preserve the observed URL spelling in quote while normalizing its identity.
                    for m in re.finditer(r"https?://[^\s<>\"')\]]+",line):
                        if value in linkedin_urls(m.group()):
                            proposals.append({'kind':'linkedin','quote':m.group().rstrip('.,;:!?'),'value':value})
                            seen.add(('linkedin',value))
                            break
            if len(proposals) >= 30:
                break
        # Date/event linkage is deliberately supplied and checked in human review;
        # footer years and unrelated publication dates never become procurement claims.
        return proposals[:30]
