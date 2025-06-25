import sys
from reportlab.lib.pagesizes import letter
from reportlab.lib.styles import getSampleStyleSheet
from reportlab.platypus import SimpleDocTemplate, Paragraph, Spacer
import markdown

if len(sys.argv) != 3:
    print("Usage: python generate_pdf.py INPUT_MD OUTPUT_PDF")
    sys.exit(1)

input_md = sys.argv[1]
output_pdf = sys.argv[2]

with open(input_md, 'r') as f:
    md_text = f.read()

html = markdown.markdown(md_text)

styles = getSampleStyleSheet()
story = []
for part in html.split('\n'):
    story.append(Paragraph(part, styles['Normal']))
    story.append(Spacer(1, 12))

doc = SimpleDocTemplate(output_pdf, pagesize=letter)
doc.build(story)
