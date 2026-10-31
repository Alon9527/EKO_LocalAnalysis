import json
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest
from eko_prospect.fetch import TrustedProxyTransport, Response


class CliTests(unittest.TestCase):
    def test_subprocess_import_list_export_and_rejection(self):
        with tempfile.TemporaryDirectory() as temp:
            base=[sys.executable,'-m','eko_prospect','--db',str(Path(temp)/'db.sqlite'),'--as-of','2026-11-01']
            def run(*args):
                return subprocess.run(base+list(args),capture_output=True,text=True)
            r=run('add-company','--name','Example','--website','https://example.com','--market','US')
            self.assertEqual(r.returncode,0,r.stderr)
            r=run('list'); self.assertEqual(r.returncode,0,r.stderr)
            self.assertEqual(json.loads(r.stdout)['qualified'],0)
            target=Path(temp)/'test.csv'
            r=run('export','--out',str(target));self.assertEqual(r.returncode,0,r.stderr)
            self.assertTrue(target.exists())
            r=run('review-company','--company','1','--status','approved','--actor','Test','--note','No evidence','--relationship','new','--angle','Test','--expected-revision','1')
            self.assertNotEqual(r.returncode,0)
            self.assertIn('Approval blocked',r.stderr)

    def test_proxy_explicit_configuration_required(self):
        with self.assertRaises(ValueError):
            TrustedProxyTransport(proxies={})


if __name__=='__main__': unittest.main()
