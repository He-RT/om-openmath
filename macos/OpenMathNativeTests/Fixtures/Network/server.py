"""Loopback-only deterministic HTTP faults. No credentials, upstreams or paid model calls."""
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
from pathlib import Path
import hashlib, json, threading, time
ROOT=Path(__file__).resolve().parent
class Handler(BaseHTTPRequestHandler):
 def log_message(self,*args): pass
 def handle_request(self):
  length=int(self.headers.get('Content-Length','0'))
  if length>1024*1024:self.send_error(413);return
  body=self.rfile.read(length)
  path=self.path.split('?',1)[0]
  print(json.dumps({'event':'request','path':path,'body_sha256':hashlib.sha256(body).hexdigest()}),flush=True)
  if path=='/timeout':
   time.sleep(2);return
  if path=='/redirect':
   self.send_response(302);self.send_header('Location',f'http://127.0.0.1:{self.server.server_port}/payload');self.send_header('Content-Length','0');self.end_headers();return
  if path.startswith('/status/'):
   self.send_error(int(path.rsplit('/',1)[1]));return
  payload=(ROOT/('data.json' if path=='/payload' else 'utf8-stream.sse')).read_bytes()
  self.send_response(200);self.send_header('Content-Type','application/json' if path=='/payload' else 'text/event-stream');self.send_header('Content-Length',str(len(payload)));self.end_headers()
  offset=0
  try:
   while offset<len(payload):
    size=(1,2,3,5,13)[offset%5];self.wfile.write(payload[offset:offset+size]);self.wfile.flush();offset+=size
  except (BrokenPipeError,ConnectionResetError):pass
 do_GET=handle_request
 do_POST=handle_request
server=ThreadingHTTPServer(('127.0.0.1',0),Handler)
server.daemon_threads=True
print(json.dumps({'event':'ready','host':'127.0.0.1','port':server.server_port}),flush=True)
server.serve_forever(poll_interval=0.02)
