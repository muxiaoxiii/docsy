#!/usr/bin/env python3
"""Debug script to analyze header detection for evidence 9."""
import subprocess
import re
import sys
from collections import defaultdict

def get_pdf_path():
    if len(sys.argv) > 1:
        return sys.argv[1]
    print(f"Usage: {sys.argv[0]} <path_to_pdf>")
    sys.exit(1)

HEADER_ZONE_MM = 25
FOOTER_ZONE_MM = 25
MM_TO_PT = 72.0 / 25.4

def normalize_text(text):
    """Simulate Rust normalize_header_footer_text."""
    # Full-width digits to half-width
    result = []
    for ch in text:
        code = ord(ch)
        if 0xFF10 <= code <= 0xFF19:  # ０-９
            result.append(chr(code - 0xFF10 + 0x30))
        else:
            result.append(ch)
    return ' '.join(''.join(result).split())

def parse_bbox_xml(xml_text):
    """Parse pdftotext -bbox XML output."""
    pages = []
    current_page = None
    for line in xml_text.split('\n'):
        page_match = re.search(r'<page width="([\d.]+)" height="([\d.]+)">', line)
        if page_match:
            current_page = {'width': float(page_match.group(1)), 'height': float(page_match.group(2)), 'words': []}
            pages.append(current_page)
            continue
        word_match = re.search(r'<word xMin="([\d.]+)" yMin="([\d.]+)" xMax="([\d.]+)" yMax="([\d.]+)">(.+?)</word>', line)
        if word_match and current_page:
            current_page['words'].append({
                'x0': float(word_match.group(1)),
                'y0': float(word_match.group(2)),
                'x1': float(word_match.group(3)),
                'y1': float(word_match.group(4)),
                'text': word_match.group(5),
            })
        if '</page>' in line and current_page:
            current_page = None
    return pages

def main():
    pdf_path = get_pdf_path()
    # Run pdftotext -bbox
    result = subprocess.run(['pdftotext', '-bbox', pdf_path, '-'], capture_output=True, text=True)
    pages = parse_bbox_xml(result.stdout)
    
    header_zone_pt = HEADER_ZONE_MM * MM_TO_PT
    footer_zone_pt = FOOTER_ZONE_MM * MM_TO_PT
    
    print(f"Total pages: {len(pages)}")
    print(f"Header zone: {HEADER_ZONE_MM}mm = {header_zone_pt:.1f}pt")
    print(f"Footer zone: {FOOTER_ZONE_MM}mm = {footer_zone_pt:.1f}pt")
    print()
    
    # Extract header candidates per page
    all_candidates = defaultdict(list)  # normalized_text -> [(page, text, y0)]
    
    for i, page in enumerate(pages):
        page_num = i + 1
        height = page['height']
        
        # Group words into lines (same y bucket)
        lines = defaultdict(list)
        for word in page['words']:
            if word['y0'] <= header_zone_pt:
                bucket = round((word['y0'] + word['y1']) / 2.0 / 3.0)
                lines[bucket].append(word)
        
        for bucket, words in sorted(lines.items()):
            words.sort(key=lambda w: w['x0'])
            text = ' '.join(w['text'] for w in words)
            norm = normalize_text(text)
            if norm and len(norm) >= 2:
                y0 = min(w['y0'] for w in words)
                all_candidates[norm].append((page_num, text, y0))
    
    # Print candidates that appear on multiple pages
    print("=== Candidates by normalized text ===")
    for norm, entries in sorted(all_candidates.items(), key=lambda x: -len(x[1])):
        pages_seen = set(e[0] for e in entries)
        count = len(pages_seen)
        if count >= 1:  # Show all
            print(f"\n  text: {norm[:60]}")
            print(f"  count: {count}, pages: {sorted(pages_seen)}")
            for page, text, y0 in entries:
                print(f"    page {page}: y0={y0:.1f}pt, text={text[:50]}")

if __name__ == '__main__':
    main()
