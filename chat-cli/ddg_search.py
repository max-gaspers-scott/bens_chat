import urllib.request
import urllib.parse
import re
from html import unescape

def search_ddg(query):
    url = 'https://html.duckduckgo.com/html/'
    data = urllib.parse.urlencode({'q': query}).encode('utf-8')
    req = urllib.request.Request(
        url, 
        data=data, 
        headers={'User-Agent': 'Mozilla/5.0 (Windows NT 10.0; Win64; x64)'}
    )
    try:
        with urllib.request.urlopen(req) as response:
            html = response.read().decode('utf-8')
            return html
    except Exception as e:
        return f"Error: {e}"

def parse_results(html):
    # Regex to find results
    # DuckDuckGo HTML result blocks typically look like:
    # <td class="result-snippet">...</td> or <a class="result__url" ...> or <a class="result__snippet" ...>
    # Let's inspect the HTML by printing a chunk or writing a robust parser.
    # In html.duckduckgo.com:
    # <div class="result results_links results_links_deep web-result ">
    #   <div class="links_main links_deep result__body">
    #     <a class="result__url" href="...">
    #     <a class="result__snippet" ...>...</a>
    # Let's extract the result blocks using a broad regex or simple string splitting.
    blocks = html.split('<div class="result results_links results_links_deep web-result')
    if len(blocks) <= 1:
        # try another split
        blocks = html.split('<div class="web-result')
    
    results = []
    for block in blocks[1:]:
        # extract title
        # <a class="result__snip" ... or similar
        title_match = re.search(r'class="result__a"[^>]*>(.*?)</a>', block, re.DOTALL)
        url_match = re.search(r'class="result__url"[^>]*>(.*?)</a>', block, re.DOTALL)
        snippet_match = re.search(r'class="result__snippet"[^>]*>(.*?)</a>', block, re.DOTALL)
        
        if title_match:
            title = re.sub(r'<[^>]+>', '', title_match.group(1)).strip()
            url_text = re.sub(r'<[^>]+>', '', url_match.group(1)).strip() if url_match else ""
            snippet = re.sub(r'<[^>]+>', '', snippet_match.group(1)).strip() if snippet_match else ""
            
            # Extract actual link href
            href_match = re.search(r'href="([^"]+)"', title_match.group(0))
            href = href_match.group(1) if href_match else ""
            if href.startswith('//'):
                href = 'https:' + href
            # Sometimes DDG uses redirect URLs: /l/?kh=-1&uddg=https%3A%2F%2F...
            if '/l/?' in href:
                parsed_href = urllib.parse.urlparse(href)
                query_params = urllib.parse.parse_qs(parsed_href.query)
                if 'uddg' in query_params:
                    href = query_params['uddg'][0]
                    
            results.append({
                'title': unescape(title),
                'url': unescape(href if href else url_text),
                'snippet': unescape(snippet)
            })
    return results

if __name__ == '__main__':
    import sys
    query = " ".join(sys.argv[1:]) if len(sys.argv) > 1 else "quebec city tourist attractions"
    print(f"Searching for: {query}")
    html = search_ddg(query)
    results = parse_results(html)
    if not results:
        print("No results found. HTML length:", len(html))
        # print some of the HTML to debug
        print(html[:2000])
    for idx, r in enumerate(results[:10], 1):
        print(f"{idx}. {r['title']}\n   URL: {r['url']}\n   Snippet: {r['snippet']}\n")
