"""Cold protocol and containment acceptance for the owned MCP coding helper."""
import json, os, pathlib, select, subprocess, sys, tempfile, time, unittest
BINARY = str(pathlib.Path(sys.argv[1]).resolve()) if len(sys.argv) > 1 else ""
sys.argv = sys.argv[:1]
class Tools(unittest.TestCase):
    def setUp(self):
        self.directory = tempfile.TemporaryDirectory(prefix="jj-store-tools-")
        self.root = pathlib.Path(self.directory.name)
        self.child = subprocess.Popen([BINARY], cwd=self.root, stdin=subprocess.PIPE,
            stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True,
            env={"PATH":"/usr/bin:/bin","HOME":str(self.root)}, start_new_session=True)
        self.sequence = 0
    def tearDown(self):
        if self.child.poll() is None:
            self.child.terminate()
            try: self.child.wait(timeout=5)
            except subprocess.TimeoutExpired: self.child.kill(); self.child.wait()
        for stream in (self.child.stdin, self.child.stdout, self.child.stderr): stream.close()
        self.directory.cleanup()
    def send(self, method, params=None):
        self.sequence += 1
        self.child.stdin.write(json.dumps({"jsonrpc":"2.0","id":self.sequence,"method":method,"params":params or {}})+"\n")
        self.child.stdin.flush()
    def receive(self, timeout=5):
        self.assertTrue(select.select([self.child.stdout],[],[],timeout)[0], "bounded response deadline")
        return json.loads(self.child.stdout.readline())["result"]
    def call(self, name, arguments):
        self.send("tools/call", {"name":name,"arguments":arguments})
        return self.receive()
    def test_cold_handshake_and_workspace_edit(self):
        self.send("initialize"); self.assertIn("tools",self.receive()["capabilities"])
        self.send("tools/list"); self.assertEqual(len(self.receive()["tools"]),4)
        self.assertNotIn("isError",self.call("write_file",{"path":"hello.txt","content":"hello"}))
        self.assertEqual(self.call("read_file",{"path":"hello.txt"})["content"][0]["text"],"hello")
        self.assertIn("hello.txt:1",self.call("search",{"query":"hello"})["content"][0]["text"])
    def test_traversal_metadata_symlink_and_bounded_files(self):
        external=self.root.parent / (self.root.name+"-outside")
        external.write_text("outside")
        try:
            (self.root/"escape").symlink_to(external)
            for path in ("../"+external.name,str(external),"escape",".jj/config"):
                self.assertTrue(self.call("read_file",{"path":path})["isError"])
            self.assertTrue(self.call("write_file",{"path":"escape","content":"bad"})["isError"])
            self.assertEqual(external.read_text(),"outside")
            (self.root/"large").write_bytes(b"a"*262145)
            self.assertTrue(self.call("read_file",{"path":"large"})["isError"])
        finally: external.unlink()
    def test_shell_deadline_and_output_budget(self):
        self.assertTrue(self.call("shell",{"command":"sleep 10","timeout_seconds":1})["isError"])
        reply=self.call("shell",{"command":"yes output | head -100000","timeout_seconds":2})
        self.assertLess(len(json.dumps(reply)),140000)
    def test_eof_cancels_shell_descendants(self):
        self.send("tools/call",{"name":"shell","arguments":{"command":"sleep 30 & echo $! > child.pid; wait","timeout_seconds":120}})
        deadline=time.time()+5
        while not (self.root/"child.pid").exists() and time.time()<deadline: time.sleep(.05)
        self.assertTrue((self.root/"child.pid").exists())
        pid=int((self.root/"child.pid").read_text())
        self.child.stdin.close(); self.child.wait(timeout=5)
        deadline=time.time()+5
        while time.time()<deadline:
            try: os.kill(pid,0)
            except ProcessLookupError: break
            time.sleep(.05)
        else: self.fail("shell descendant survived EOF")
    def test_oversized_frame_stops_without_response(self):
        self.child.stdin.write(" "*524289); self.child.stdin.flush()
        self.child.wait(timeout=5); self.assertEqual(self.child.stdout.read(),"")
if __name__ == "__main__": unittest.main()
