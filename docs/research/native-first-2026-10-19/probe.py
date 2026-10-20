"""Synthetic test fixtures and independent qpdf/Poppler measurements; no user PDFs."""
import json
import re
import subprocess
import sys
from pathlib import Path

ROOT = Path(sys.argv[2]).resolve()
PYMODE = sys.argv[1]

def run(args):
    p = subprocess.run(args, capture_output=True, text=True, timeout=40)
    return {"args": list(map(str, args)), "code": p.returncode, "stdout": p.stdout, "stderr": p.stderr}

def stream(data, extra=b""):
    return b"<< /Length " + str(len(data)).encode() + b" " + extra + b" >>\nstream\n" + data + b"\nendstream"

def make_pdf(annot, *, state=False, ap=True, matrix=False, stale=False):
    page_extra = b" /UserUnit 2 /CropBox [20 30 380 370] /Rotate 90" if matrix else b""
    annotation = b"<< /Type /Annot /Rect [100 100 200 160] /P 3 0 R " + annot
    if ap:
        annotation += b" /AP << /N " + (b"<< /Yes 6 0 R /Off 8 0 R >>" if state else b"6 0 R") + b" >>"
    annotation += b" >>"
    objects = [
        b"<< /Type /Catalog /Pages 2 0 R" + (b" /AcroForm 7 0 R" if b"/Widget" in annot else b"") + b" >>",
        b"<< /Type /Pages /Kids [3 0 R] /Count 1 /MediaBox [0 0 400 400] >>",
        b"<< /Type /Page /Parent 2 0 R /Resources << >> /Contents 4 0 R /Annots [5 0 R]" + page_extra + b" >>",
        stream(b"0 0 1 rg 20 20 40 40 re f"), annotation,
        stream(b"1 0 0 rg 0 0 100 60 re f", b"/Type /XObject /Subtype /Form /BBox [0 0 100 60] /Resources << >>" + (b" /Matrix [0 1 -1 0 60 0]" if matrix else b"")),
        b"<< /Fields [5 0 R] /NeedAppearances " + (b"true" if stale else b"false") + b" >>",
        stream(b"0 1 0 rg 0 0 100 60 re f", b"/Type /XObject /Subtype /Form /BBox [0 0 100 60] /Resources << >>"),
    ]
    data = bytearray(b"%PDF-1.7\n%\xe2\xe3\xcf\xd3\n")
    offsets = [0]
    for n, obj in enumerate(objects, 1):
        offsets.append(len(data))
        data += f"{n} 0 obj\n".encode() + obj + b"\nendobj\n"
    xref = len(data)
    data += f"xref\n0 {len(objects)+1}\n0000000000 65535 f \n".encode()
    data += b"".join(f"{n:010d} 00000 n \n".encode() for n in offsets[1:])
    data += f"trailer\n<< /Size {len(objects)+1} /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF\n".encode()
    return bytes(data)

if PYMODE == "generate":
    for folder in ("inputs", "qpdf", "native", "renders"):
        (ROOT / folder).mkdir(parents=True, exist_ok=True)
    cases = {
        "normal_widget": (b"/Subtype /Widget /FT /Tx /T (field) /V (value) /F 4", {}),
        "as_checkbox": (b"/Subtype /Widget /FT /Btn /T (check) /V /Yes /AS /Yes /F 4", {"state": True}),
        "missing_ap": (b"/Subtype /Widget /FT /Tx /T (field) /V (value) /F 4", {"ap": False}),
        "no_view": (b"/Subtype /Widget /FT /Tx /T (field) /V (value) /F 36", {}),
        "invisible": (b"/Subtype /Widget /FT /Tx /T (field) /V (value) /F 5", {}),
        "hidden": (b"/Subtype /Widget /FT /Tx /T (field) /V (value) /F 6", {}),
        "watermark": (b"/Subtype /Watermark /F 4", {}),
        "rotated_ap": (b"/Subtype /Widget /FT /Tx /T (field) /V (value) /F 4", {"matrix": True}),
        "zero_bbox": (b"/Subtype /Widget /FT /Tx /T (field) /V (value) /F 4", {}),
        "stale_widget": (b"/Subtype /Widget /FT /Tx /T (field) /V (NEW) /F 4", {"stale": True}),
    }
    for name, (annot, options) in cases.items():
        data = make_pdf(annot, **options)
        if name == "zero_bbox":
            # Same byte length, so xref offsets remain valid.
            data = data.replace(b"/BBox [0 0 100 60]", b"/BBox [0 0 000 60]")
        (ROOT / "inputs" / f"{name}.pdf").write_bytes(data)
    baseline = ROOT / "inputs/normal_widget.pdf"
    commands = []
    for name, bits, aes, user in [("rc4",40,False,"user"),("aes128",128,True,"user"),("aes256",256,True,"user"),("empty_aes256",256,True,"")]:
        out = ROOT / "inputs" / f"crypto_{name}.pdf"
        args = ["qpdf", "--allow-weak-crypto", "--encrypt", user, "owner", str(bits)]
        if bits == 128:
            args.append("--use-aes=y")
        commands.append(run(args + ["--", str(baseline), str(out)]))
    raw = baseline.read_bytes()
    (ROOT / "inputs/repair_bad_startxref.pdf").write_bytes(re.sub(rb"startxref\n\d+", b"startxref\n0", raw))
    (ROOT / "inputs/repair_missing_xref.pdf").write_bytes(raw[:raw.index(b"xref\n")] + b"trailer\n<< /Root 1 0 R /Size 9 >>\n%%EOF\n")
    (ROOT / "generation.json").write_text(json.dumps(commands, indent=2))
    print(f"Generated {len(list((ROOT/'inputs').glob('*.pdf')))} synthetic input PDFs")

elif PYMODE == "measure":
    from pypdf import PdfReader
    from PIL import Image, ImageChops, ImageOps, ImageDraw
    results = []
    def describe(path):
        r = PdfReader(path)
        page = r.pages[0]
        annotations = [a.get_object() for a in page.get("/Annots", [])]
        af = r.trailer["/Root"].get("/AcroForm")
        return {"annotations": len(annotations), "widgets": sum(a.get("/Subtype") == "/Widget" for a in annotations), "fields": len(af.get_object().get("/Fields", [])) if af else 0}
    for source in sorted((ROOT / "inputs").glob("*.pdf")):
        if source.stem.startswith("crypto_"):
            continue
        target = ROOT / "qpdf" / source.name
        cmd = run(["qpdf", "--flatten-annotations=all", str(source), str(target)])
        entry = {"case":source.stem, "qpdf":cmd, "documents":{}}
        for engine, path in [("source",source),("native",ROOT/"native"/source.name),("qpdf",target)]:
            if not path.exists():
                continue
            try:
                entry["documents"][engine] = describe(path)
            except Exception as error:
                entry["documents"][engine] = {"reader_error": str(error)}
            png = ROOT / "renders" / f"{source.stem}-{engine}"
            entry["documents"][engine]["render"] = run(["pdftoppm","-r","72","-singlefile","-png",str(path),str(png)])["code"]
            if not Path(str(png)+".png").exists():
                continue
            im = Image.open(str(png)+".png").convert("RGB")
            entry["documents"][engine]["size"] = list(im.size)
            entry["documents"][engine]["red_pixels"] = sum(1 for r,g,b in im.get_flattened_data() if r>200 and g<70 and b<70)
        results.append(entry)
    # Each test case appears as source/native/qpdf in a row.
    sheet = Image.new("RGB", (900, 240*len(results)), "#eeeeee")
    draw = ImageDraw.Draw(sheet)
    for row, entry in enumerate(results):
        for col, engine in enumerate(["source","native","qpdf"]):
            path = ROOT / "renders" / f"{entry['case']}-{engine}.png"
            if path.exists():
                im = ImageOps.contain(Image.open(path).convert("RGB"), (280,205))
                sheet.paste(im, (col*300, row*240+25))
            draw.text((col*300+5,row*240+5), f"{entry['case']} / {engine}", fill="black")
    sheet.save(ROOT/"contact-sheet.png")
    (ROOT/"measurements.json").write_text(json.dumps(results,ensure_ascii=False,indent=2))
    print(json.dumps([{ "case":r["case"], **{k:{a:b for a,b in v.items() if a!="render"} for k,v in r["documents"].items()} } for r in results],indent=2))
