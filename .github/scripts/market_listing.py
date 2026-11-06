"""Create or update a port's Pumpkin Market listing from plugins/<id>/ace.json.

usage: python market_listing.py <plugin id>        (token in env PUMPKIN_MARKET_TOKEN)

Reads ace.json -> market: { id?, category, keywords, commands[] } plus name, description and original, and
plugins/<id>/icon.png. Without market.id it creates a draft first and prints the new id: put it in ace.json.
Writes MARKET_ID=<id> to $GITHUB_OUTPUT when run in Actions.
"""
import json, os, sys, uuid, urllib.request, urllib.error, pathlib

API = 'https://market.pumpkinmc.org/api'
REPO = 'https://github.com/ashfaqizop/ace-studio'
CATEGORIES = {'Admin Tools', 'Economy', 'Fun', 'World Management', 'Utilities', 'Chat', 'Other'}
TOKEN = os.environ.get('PUMPKIN_MARKET_TOKEN', '')
if not TOKEN:
    sys.exit('PUMPKIN_MARKET_TOKEN is not set')

pid_name = sys.argv[1]
if not pid_name or not all(c.islower() or c.isdigit() or c == '-' for c in pid_name):
    sys.exit(f'bad plugin id {pid_name!r}')
root = pathlib.Path(__file__).resolve().parents[2]
port = root / 'plugins' / pid_name
manifest = json.loads((port / 'ace.json').read_text(encoding='utf-8'))
market = manifest.get('market') or {}


def call(method, path, body=None, headers=None):
    # Cloudflare rejects urllib's default User-Agent (error 1010)
    h = {'Authorization': f'Bearer {TOKEN}', 'X-Requested-With': 'XMLHttpRequest', 'User-Agent': 'curl/8.9.1',
         'Accept': 'application/json', **(headers or {})}
    req = urllib.request.Request(API + path, data=body, method=method, headers=h)
    try:
        with urllib.request.urlopen(req) as r:
            text = r.read().decode() or '{}'
            return r.status, json.loads(text) if text.strip().startswith(('{', '[')) else text
    except urllib.error.HTTPError as e:
        return e.code, e.read().decode()[:500]


def multipart(fields, files):
    b = uuid.uuid4().hex
    out = bytearray()
    for k, v in fields.items():
        out += f'--{b}\r\nContent-Disposition: form-data; name="{k}"\r\n\r\n{v}\r\n'.encode()
    for k, (name, data, ctype) in files.items():
        out += f'--{b}\r\nContent-Disposition: form-data; name="{k}"; filename="{name}"\r\nContent-Type: {ctype}\r\n\r\n'.encode()
        out += data + b'\r\n'
    out += f'--{b}--\r\n'.encode()
    return bytes(out), f'multipart/form-data; boundary={b}'


category = market.get('category', 'Other')
if category not in CATEGORIES:
    sys.exit(f'market.category must be one of {sorted(CATEGORIES)}')

pid = market.get('id')
if not pid:
    status, res = call('POST', '/plugins/draft', json.dumps({'name': manifest['name'], 'category': category, 'type': 'free'}).encode(),
                       {'Content-Type': 'application/json'})
    print('draft:', status, res if status >= 300 else {'id': res.get('id')})
    if status >= 300:
        sys.exit(1)
    pid = res['id']

source = f'{REPO}/tree/main/plugins/{pid_name}'
original = manifest['original']
description = (
    f"> **{manifest['name']}** is an Ace Studio port of [{original['name']}]({original['url']}) to Pumpkin. "
    f"Open source ({manifest['license']}): {source}\n\n" + manifest['description']
)
meta = {
    'name': manifest['name'],
    'category': category,
    'sourceLink': source,
    'keywords': market.get('keywords', ''),
    'translatedDescriptions': {'en': description},
    'isEarlyAccess': False,
    'commands': [
        {'name': c['name'], 'aliases': c.get('aliases', []), 'permission': c.get('permission', ''), 'display_order': i,
         'description': {'en': c['description']}}
        for i, c in enumerate(market.get('commands', []))
    ],
}
fields = {'metadata': json.dumps(meta)}
icon = port / 'icon.png'
files = {'preview_image': ('icon.png', icon.read_bytes(), 'image/png')} if icon.exists() else {}
body, ctype = multipart(fields, files)
status, res = call('PUT', f'/plugins/{pid}', body, {'Content-Type': ctype})
print('details:', status, '' if status < 300 else res)
if status >= 300:
    sys.exit(1)
status, res = call('GET', f'/plugins/{pid}/readiness')
print('readiness:', status, res)

print(f'MARKET_ID={pid}')
if os.environ.get('GITHUB_OUTPUT'):
    with open(os.environ['GITHUB_OUTPUT'], 'a') as f:
        f.write(f'MARKET_ID={pid}\n')
