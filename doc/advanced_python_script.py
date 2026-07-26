# /// script
# requires-python = ">=3.10"
# dependencies = [
#     "docling",
# ]
# ///

import glob
from pathlib import Path
from docling.document_converter import DocumentConverter

def main():
    print("Initializing Docling models (this may take a moment)...")
    converter = DocumentConverter()
    
    output_dir = Path("md_output")
    output_dir.mkdir(exist_ok=True)
    
    # Sort files by size so the smaller Technical Notes process first
    pdf_files = sorted(glob.glob("*.pdf"), key=os.path.getsize)
    
    for pdf_path in pdf_files:
        print(f"\nProcessing {pdf_path}...")
        try:
            result = converter.convert(pdf_path)
            md_content = result.document.export_to_markdown()
            
            out_file = output_dir / f"{Path(pdf_path).stem}.md"
            with open(out_file, "w", encoding="utf-8") as f:
                f.write(md_content)
                
            print(f"Successfully saved to {out_file}")
        except Exception as e:
            print(f"Error converting {pdf_path}: {e}")

if __name__ == "__main__":
    import os
    main()