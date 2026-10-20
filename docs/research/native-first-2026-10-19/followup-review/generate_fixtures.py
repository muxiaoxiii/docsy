from pathlib import Path
import subprocess
p=Path(Path('/tmp/docsy-native-followup-review-path').read_text());d=p/'inputs';d.mkdir(exist_ok=True)
def stream(b,extra=b''):return b'<< /Length '+str(len(b)).encode()+b' '+extra+b' >>\nstream\n'+b+b'\nendstream'
def pdf(name,*,parent=b'',page=b'',body=b'0 0 1 rg 20 30 160 160 re f',annot=None,ap=None,field=False,stale=False):
 objects=[b'<< /Type /Catalog /Pages 2 0 R'+(b' /AcroForm << /Fields [5 0 R] /NeedAppearances '+(b'true' if stale else b'false')+b' >>' if field else b'')+b' >>',b'<< /Type /Pages /Kids [3 0 R] /Count 1 /MediaBox [0 0 200 220] '+parent+b' >>',b'<< /Type /Page /Parent 2 0 R /Resources << >> /Contents 4 0 R '+page+(b' /Annots [5 0 R]' if annot else b'')+b' >>',stream(body)]
 if annot:objects.append(b'<< /Type /Annot /P 3 0 R '+annot+(b' /AP << /N 6 0 R >>' if ap else b'')+b' >>')
 if ap:objects.append(stream(ap,b'/Type /XObject /Subtype /Form /BBox [0 0 100 40] /Resources << >>'))
 data=bytearray(b'%PDF-1.7\n');offsets=[0]
 for i,o in enumerate(objects,1):offsets.append(len(data));data+=f'{i} 0 obj\n'.encode()+o+b'\nendobj\n'
 off=len(data);data+=f'xref\n0 {len(objects)+1}\n0000000000 65535 f \n'.encode()+b''.join(f'{v:010d} 00000 n \n'.encode() for v in offsets[1:])+f'trailer\n<< /Size {len(objects)+1} /Root 1 0 R >>\nstartxref\n{off}\n%%EOF\n'.encode();(d/(name+'.pdf')).write_bytes(data)
pdf('unit_leaf',page=b'/UserUnit 2 /CropBox [20 30 180 190]')
pdf('unit_parent',parent=b'/UserUnit 2',page=b'/CropBox [20 30 180 190]')
pdf('highlight_no_ap',body=b'0 0 1 rg 20 20 10 10 re f',annot=b'/Subtype /Highlight /Rect [50 60 150 90] /QuadPoints [50 90 150 90 50 60 150 60] /C [1 1 0] /CA 1 /F 4')
ap=b'1 0 0 rg 0 0 100 40 re f 0 1 0 rg 0 0 20 40 re f'
pdf('stamp_opacity',body=b'',annot=b'/Subtype /Stamp /Rect [50 60 150 100] /F 4 /CA 0.2',ap=ap)
pdf('stamp_norotate',body=b'',page=b'/Rotate 90',annot=b'/Subtype /Stamp /Rect [50 60 150 100] /F 20',ap=ap)
pdf('stale_widget',body=b'',annot=b'/Subtype /Widget /FT /Tx /T (field) /V (NEW) /Rect [50 60 150 100] /F 4',ap=ap,field=True,stale=True)
pdf('bookmark_plain')
r=subprocess.run(['/opt/homebrew/bin/qpdf','--encrypt','','owner','256','--modify=none','--',str(d/'bookmark_plain.pdf'),str(d/'bookmark_encrypted.pdf')],capture_output=True,text=True);assert r.returncode==0,r.stderr
for f in d.glob('*.pdf'):
 r=subprocess.run(['/opt/homebrew/bin/qpdf','--flatten-annotations=all',str(f),str(p/'outputs'/(f.stem+'-qpdf.pdf'))],capture_output=True,text=True)
(p/'fixtures-ready.txt').write_text('Synthetic fixtures only. No private documents.\n')
