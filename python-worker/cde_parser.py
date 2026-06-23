import re
import datetime
import os

try:
    import fitz # PyMuPDF
except ImportError:
    fitz = None

try:
    import docx
except ImportError:
    docx = None

def extract_text_from_pdf(file_path):
    if not fitz:
        # Fallback raw scan
        try:
            with open(file_path, 'rb') as f:
                content = f.read()
                # strip non-ascii characters to get printable text
                return "".join(chr(c) for c in content if 32 <= c < 127 || c in (10, 13))
        except Exception:
            return ""
    try:
        doc = fitz.open(file_path)
        text = ""
        for page in doc:
            text += page.get_text()
        return text
    except Exception as e:
        print(f"Error reading PDF {file_path}: {e}")
        return ""

def extract_text_from_docx(file_path):
    if not docx:
        return ""
    try:
        doc = docx.Document(file_path)
        return "\n".join([para.text for para in doc.paragraphs])
    except Exception as e:
        print(f"Error reading DOCX {file_path}: {e}")
        return ""

def extract_text_from_file(file_path):
    ext = os.path.splitext(file_path)[1].lower()
    if ext == '.pdf':
        return extract_text_from_pdf(file_path)
    elif ext in ('.docx', '.doc'):
        return extract_text_from_docx(file_path)
    else:
        try:
            with open(file_path, 'r', encoding='utf-8', errors='ignore') as f:
                return f.read()
        except Exception:
            return ""

def parse_commitments(file_path):
    text = extract_text_from_file(file_path)
    if not text:
        return []

    commitments = []
    
    # Common date regex formats
    # e.g., "due July 15", "due on 2026-07-15", "deadline: 07/15/2026"
    date_patterns = [
        r'(?:due\s+on|due|deadline|submit\s+by|scheduled\s+for)\s*[:\-]?\s*([A-Za-z]+ \d{1,2}(?:\s*,\s*\d{4})?)', # due July 15, due July 15, 2026
        r'(?:due\s+on|due|deadline|submit\s+by)\s*[:\-]?\s*(\d{4}-\d{2}-\d{2})', # due 2026-07-15
        r'(?:due\s+on|due|deadline|submit\s+by)\s*[:\-]?\s*(\d{1,2}/\d{1,2}/\d{2,4})', # due 07/15/2026
    ]
    
    normalized_deadline = None
    confidence_score = 0.5
    
    for pattern in date_patterns:
        matches = re.findall(pattern, text, re.IGNORECASE)
        if matches:
            raw_date = matches[0].strip()
            # Normalize to ISO-8601
            normalized_deadline = normalize_date(raw_date)
            if normalized_deadline:
                confidence_score = 0.85
                break
                
    # If no deadline found, try to extract any date near assignment keywords
    if not normalized_deadline:
        date_pattern = r'(\d{4}-\d{2}-\d{2}|\d{1,2}/\d{1,2}/\d{2,4})'
        all_dates = re.findall(date_pattern, text)
        if all_dates:
            normalized_deadline = normalize_date(all_dates[0])
            confidence_score = 0.60

    if normalized_deadline:
        # Determine title and type
        title = os.path.basename(file_path)
        commitment_type = "ASSIGNMENT"
        
        # Heuristics for type
        text_lower = text.lower()
        if "meeting" in text_lower or "sync" in text_lower or "call" in text_lower:
            commitment_type = "MEETING"
        elif "milestone" in text_lower or "deliverable" in text_lower or "ship" in text_lower:
            commitment_type = "DELIVERABLE"
        elif "exam" in text_lower or "quiz" in text_lower or "test" in text_lower:
            commitment_type = "OBLIGATION"
            
        # Clean title name from file
        title_clean = os.path.splitext(title)[0].replace("_", " ").replace("-", " ")
        
        commitments.append({
            "title": title_clean,
            "commitment_type": commitment_type,
            "deadline_date": normalized_deadline,
            "confidence_score": confidence_score
        })

    return commitments

def normalize_date(raw_date):
    # Try parsing different formats
    formats = [
        ("%Y-%m-%d", None),
        ("%m/%d/%Y", None),
        ("%m/%d/%y", None),
        ("%B %d", True), # e.g. July 15 (assume current year)
        ("%b %d", True), # e.g. Jul 15 (assume current year)
        ("%B %d, %Y", None),
        ("%b %d, %Y", None),
    ]
    
    current_year = datetime.datetime.now().year
    
    for fmt, needs_year in formats:
        try:
            dt = datetime.datetime.strptime(raw_date, fmt)
            if needs_year:
                dt = dt.replace(year=current_year)
            return dt.strftime("%Y-%m-%d")
        except ValueError:
            continue
            
    return None
