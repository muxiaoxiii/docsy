"""Synthetic PDF research fixtures; intentionally outside Docsy's production code."""
import argparse
from io import BytesIO
from pathlib import Path

from pypdf import PdfReader, PdfWriter
from pypdf.annotations import Link
from reportlab.pdfbase import pdfmetrics
from reportlab.pdfbase.cidfonts import UnicodeCIDFont
from reportlab.pdfgen import canvas


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--out", type=Path, required=True)
    args = parser.parse_args()
    args.out.mkdir(parents=True, exist_ok=True)
    targets = [args.out / "input-A.pdf", args.out / "input-B.pdf"]
    if any(path.exists() for path in targets):
        raise FileExistsError("Use a fresh output directory; existing fixtures are preserved")
    pdfmetrics.registerFont(UnicodeCIDFont("STSong-Light"))
    for label, count in [("A", 2), ("B", 1)]:
        buffer = BytesIO()
        page = canvas.Canvas(buffer, pagesize=(595, 842))
        for index in range(count):
            page.setFont("Helvetica", 20)
            page.drawString(60, 760, f"SOURCE {label} PAGE {index + 1}")
            page.setFont("STSong-Light", 16)
            page.drawString(60, 720, "中文证据测试：页序与内容保持")
            page.setStrokeColorRGB(0.1, 0.5, 0.2)
            page.rect(50, 650, 490, 130)
            if index == 0:
                page.acroForm.textfield(
                    name="common_name", x=70, y=600, width=200, height=25,
                    value=f"value-{label}", forceBorder=True,
                )
            page.showPage()
        page.save()
        writer = PdfWriter()
        writer.clone_document_from_reader(PdfReader(buffer))
        if count == 2:
            writer.pages[1].rotate(90)
            writer.add_annotation(
                page_number=1,
                annotation=Link(rect=(60, 700, 300, 740), target_page_index=0),
            )
        writer.add_outline_item(f"Source {label} first", 0)
        writer.add_attachment(f"{label}.txt", f"attachment-{label}".encode())
        writer.add_metadata({
            "/Title": f"Fixture {label}",
            "/DocsyReviewToken": "must-not-be-assumed-preserved",
        })
        with (args.out / f"input-{label}.pdf").open("xb") as output:
            writer.write(output)


if __name__ == "__main__":
    main()
