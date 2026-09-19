#!/usr/bin/env python3
"""Check local Markdown links and heading targets.

Situational check for ADR 0075 document packets. This script makes no network requests.
It supports inline links, reference links, Markdown headings, and explicit HTML anchors.
Use fenced code blocks in checked documents.
"""

import argparse
import html
from pathlib import Path
import re
import sys
import unicodedata
from urllib.parse import unquote, urlsplit


DESTINATION = r'<([^>\n]+)>|((?:\\.|[^\s\\()]|\([^()\n]*\))+)'
INLINE = re.compile(
    r'!?\[(?:\\.|[^\]\\])*\]\(\s*(?:' + DESTINATION
    + r''')(?:\s+(?:"[^"\n]*"|'[^'\n]*'|\([^\n]*\)))?\s*\)'''
)
DEFINITION = re.compile(r'^ {0,3}\[([^]\n]+)\]:\s*(?:' + DESTINATION + r')', re.M)
REFERENCE = re.compile(r'!?\[([^]\n]+)\]\[([^]\n]*)\]')
SHORT_REFERENCE = re.compile(r'!?\[([^]\n]+)\]')
HTML_ANCHOR = re.compile(r'''\b(?:id|name)\s*=\s*["']([^"']+)["']''')


def prose(text):
    """Remove fenced code while preserving line numbers."""
    result = []
    fence = None
    for line in text.splitlines(keepends=True):
        match = re.match(r'^ {0,3}(`{3,}|~{3,})', line)
        if fence:
            if match and match[1][0] == fence[0] and len(match[1]) >= len(fence):
                fence = None
            result.append('\n')
        elif match:
            fence = match[1]
            result.append('\n')
        else:
            result.append(line)
    return ''.join(result)


def reference_key(label):
    return ' '.join(label.split()).casefold()


def heading_slug(heading):
    heading = re.sub(r'!?\[([^]]+)\]\([^)]*\)', r'\1', heading)
    heading = re.sub(r'<[^>]*>', '', heading)
    heading = html.unescape(heading).lower().replace('`', '')
    return ''.join(
        character for character in heading
        if character in ' _-' or unicodedata.category(character)[0] in 'LMN'
    ).replace(' ', '-')


def anchors(path):
    text = prose(path.read_text(encoding='utf-8'))
    html_text = re.sub(r'(`+)([^`\n]*?)\1', '', text)
    html_text = re.sub(r'<!--.*?-->', '', html_text, flags=re.S)
    result = {
        value for tag in re.findall(r'<[A-Za-z][^>]*>', html_text)
        for value in HTML_ANCHOR.findall(tag)
    }
    generated = set()
    lines = text.splitlines()
    for index, line in enumerate(lines):
        match = re.match(r'^ {0,3}#{1,6}\s+(.+?)\s*$', line)
        if match:
            heading = re.sub(r'\s+#+\s*$', '', match[1])
        elif index and re.fullmatch(r' {0,3}(?:=+|-+)\s*', line):
            heading = lines[index - 1].strip()
            if not heading or heading.startswith('|'):
                continue
        else:
            continue
        base = heading_slug(heading)
        slug = base
        suffix = 0
        while slug in generated:
            suffix += 1
            slug = f'{base}-{suffix}'
        generated.add(slug)
        result.add(slug)
    return result


def links(text):
    text = prose(text)
    text = re.sub(r'(`+)([^`\n]*?)\1', lambda match: ' ' * len(match[0]), text)
    definitions = {}
    for match in DEFINITION.finditer(text):
        definitions.setdefault(reference_key(match[1]), match[2] or match[3])
    for match in INLINE.finditer(text):
        yield text.count('\n', 0, match.start()) + 1, match[1] or match[2]
    remaining = INLINE.sub(lambda match: ' ' * len(match[0]), text)
    remaining = DEFINITION.sub(lambda match: ' ' * len(match[0]), remaining)
    for match in REFERENCE.finditer(remaining):
        key = reference_key(match[2] or match[1])
        yield remaining.count('\n', 0, match.start()) + 1, definitions.get(key)
    remaining = REFERENCE.sub(lambda match: ' ' * len(match[0]), remaining)
    for match in SHORT_REFERENCE.finditer(remaining):
        key = reference_key(match[1])
        if key in definitions:
            yield remaining.count('\n', 0, match.start()) + 1, definitions[key]


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('files', nargs='+', type=Path)
    args = parser.parse_args()
    errors = []
    anchor_cache = {}
    count = 0
    for path in args.files:
        try:
            text = path.read_text(encoding='utf-8')
        except (OSError, UnicodeError) as error:
            errors.append(f'{path}: Cannot read input: {error}')
            continue
        for line, target in links(text):
            if target is None:
                errors.append(f'{path}:{line}: Missing reference definition')
                continue
            target = html.unescape(re.sub(r'\\([\\() ])', r'\1', target))
            parts = urlsplit(target)
            if parts.scheme or parts.netloc:
                continue
            count += 1
            destination = (path.parent / unquote(parts.path)).resolve() if parts.path else path.resolve()
            if not destination.exists():
                errors.append(f'{path}:{line}: Missing file: {target}')
                continue
            if parts.fragment:
                try:
                    if destination not in anchor_cache:
                        anchor_cache[destination] = anchors(destination)
                    if unquote(parts.fragment) not in anchor_cache[destination]:
                        errors.append(f'{path}:{line}: Missing heading or anchor: {target}')
                except (OSError, UnicodeError) as error:
                    errors.append(f'{path}:{line}: Cannot check anchor: {error}')
    if errors:
        print('\n'.join(errors), file=sys.stderr)
        return 1
    print(f'Green: {count} local links in {len(args.files)} files')
    return 0


if __name__ == '__main__':
    raise SystemExit(main())
