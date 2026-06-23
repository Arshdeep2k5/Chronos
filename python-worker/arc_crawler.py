import requests
import urllib.parse
import xml.etree.ElementTree as ET

def query_arxiv(query_text):
    results = []
    try:
        encoded_query = urllib.parse.quote(query_text)
        url = f"http://export.arxiv.org/api/query?search_query=all:{encoded_query}&max_results=3"
        response = requests.get(url, timeout=10)
        if response.status_code == 200:
            root = ET.fromstring(response.content)
            # Namespace parsing
            ns = {'atom': 'http://www.w3.org/2005/Atom'}
            for entry in root.findall('atom:entry', ns):
                title = entry.find('atom:title', ns)
                summary = entry.find('atom:summary', ns)
                id_uri = entry.find('atom:id', ns)
                
                title_text = title.text.strip().replace("\n", " ") if title is not None else "Unknown Paper"
                summary_text = summary.text.strip().replace("\n", " ") if summary is not None else ""
                url_text = id_uri.text.strip() if id_uri is not None else ""
                
                results.append({
                    "source": "arXiv",
                    "title": title_text,
                    "summary": summary_text[:200] + "..." if len(summary_text) > 200 else summary_text,
                    "url": url_text,
                    "relevance_score": 0.85
                })
    except Exception as e:
        print(f"arXiv query error: {e}")
    return results

def query_github(query_text):
    results = []
    try:
        encoded_query = urllib.parse.quote(query_text)
        url = f"https://api.github.com/search/repositories?q={encoded_query}&per_page=3"
        headers = {'User-Agent': 'ChronosPilot-v1.0'}
        response = requests.get(url, headers=headers, timeout=10)
        if response.status_code == 200:
            data = response.json()
            for item in data.get('items', []):
                results.append({
                    "source": "GitHub",
                    "title": item.get('full_name', 'Unknown Repository'),
                    "summary": item.get('description') or "No description available",
                    "url": item.get('html_url', ''),
                    "relevance_score": 0.80
                })
    except Exception as e:
        print(f"GitHub query error: {e}")
    return results

def generate_briefing(query_text):
    arxiv_results = query_arxiv(query_text)
    github_results = query_github(query_text)
    return arxiv_results + github_results
