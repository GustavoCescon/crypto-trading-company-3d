"""Black-box HTTP smoke tests; no exchange keys or third-party packages."""
import json
import os
import time
import unittest
from urllib.request import urlopen
from urllib.error import HTTPError

BASE = os.environ.get("API_URL", "http://localhost:8080")


class SystemSmokeTests(unittest.TestCase):
    def get_json(self, path):
        with urlopen(BASE + path, timeout=5) as response:
            self.assertEqual(response.status, 200)
            return json.load(response)

    def test_dependencies_ready(self):
        self.assertEqual(self.get_json("/health/ready"), {
            "ready": True, "database": True, "redis": True,
        })

    def test_paper_only(self):
        status = self.get_json("/api/v1/system")
        self.assertEqual(status["trading_mode"], "simulated")
        self.assertFalse(status["execution_enabled"])
        with self.assertRaises(HTTPError) as error:
            urlopen(BASE + "/api/v1/orders", timeout=5)
        self.assertEqual(error.exception.code, 404)

    def test_startup_audited(self):
        events = self.get_json("/api/v1/events")["events"]
        self.assertTrue(any(e["payload"]["type"] == "service_started" for e in events))
        self.assertTrue(all(e["schema_version"] == 1 for e in events))

    def test_metrics(self):
        for _ in range(15):
            with urlopen(BASE + "/metrics", timeout=5) as response:
                metrics = response.read().decode()
            line = next(line for line in metrics.splitlines() if line.startswith("ctc_outbox_published_total "))
            if int(line.split()[1]) >= 1:
                return
            time.sleep(1)
        self.fail("startup event was not published to Redis")
