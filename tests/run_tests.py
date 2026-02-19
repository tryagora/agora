#!/usr/bin/env python3
"""
agora test suite runner
orchestrates load tests with docker log monitoring
works on both windows and linux
"""

import os
import sys
import subprocess
import argparse
import tempfile
import time
import signal
from pathlib import Path
from typing import List, Tuple, Optional

# configuration defaults
default_api_url = "http://localhost:3000"
default_homeserver = "http://localhost:8448"

# get script directory for relative paths
script_dir = Path(__file__).parent.resolve()
load_tests_dir = script_dir / "load-tests"


def check_services(api_url: str, homeserver: str) -> bool:
    """check if services are running"""
    print("checking if services are running...")
    
    # check api
    try:
        import urllib.request
        req = urllib.request.Request(f"{api_url}/health", method="GET")
        with urllib.request.urlopen(req, timeout=5) as response:
            if response.status == 200:
                print("   api is responding")
            else:
                print(f"   api returned status {response.status}")
                return False
    except Exception as e:
        print(f"   api is not responding at {api_url}")
        print(f"      error: {e}")
        print("      start it with: cargo run --manifest-path backend/api/Cargo.toml")
        return False
    
    # check conduit
    try:
        import urllib.request
        req = urllib.request.Request(f"{homeserver}/_matrix/client/versions", method="GET")
        with urllib.request.urlopen(req, timeout=5) as response:
            if response.status == 200:
                print("   conduit is responding")
            else:
                print(f"   conduit returned status {response.status}")
                return False
    except Exception as e:
        print(f"   conduit is not responding at {homeserver}")
        print(f"      error: {e}")
        print("      start it with: docker-compose up conduit")
        return False
    
    print("")
    return True


class DockerLogMonitor:
    """monitor docker logs in real-time"""
    
    def __init__(self, services: List[str]):
        self.services = services
        self.processes: List[subprocess.Popen] = []
        self.log_dir = None
        
    def start(self):
        """start monitoring logs"""
        # create temp directory for log files
        self.log_dir = Path(tempfile.mkdtemp())
        print(f"   log files: {self.log_dir}")
        
        # start log collection for each service
        for service in self.services:
            log_file = self.log_dir / f"{service}.log"
            container_name = f"agora_{service}" if not service.startswith("agora_") else service
            
            try:
                # use powershell on windows, bash on linux
                if sys.platform == "win32":
                    # windows: use docker logs with file redirection
                    cmd = f'docker logs -f --tail=50 {container_name} > "{log_file}" 2>&1'
                    proc = subprocess.Popen(
                        cmd,
                        shell=True,
                        stdout=subprocess.PIPE,
                        stderr=subprocess.PIPE,
                        creationflags=subprocess.CREATE_NEW_PROCESS_GROUP
                    )
                else:
                    # linux/mac
                    with open(log_file, 'w') as f:
                        proc = subprocess.Popen(
                            ["docker", "logs", "-f", "--tail=50", container_name],
                            stdout=f,
                            stderr=subprocess.STDOUT
                        )
                
                self.processes.append(proc)
            except Exception as e:
                print(f"   warning: could not start log monitoring for {service}: {e}")
        
        print(f"   started monitoring docker logs for: {', '.join(self.services)}")
    
    def stop(self):
        """stop monitoring"""
        for proc in self.processes:
            try:
                if sys.platform == "win32":
                    proc.terminate()
                else:
                    proc.terminate()
                    proc.wait(timeout=2)
            except:
                pass
    
    def print_recent_logs(self, lines: int = 50):
        """print recent logs from files"""
        print(f"\nrecent docker logs (last {lines} lines per service):")
        print("-" * 70)
        
        if not self.log_dir:
            return
            
        for service in self.services:
            log_file = self.log_dir / f"{service}.log"
            if log_file.exists():
                print(f"\n{service}:")
                try:
                    with open(log_file, 'r', encoding='utf-8', errors='ignore') as f:
                        content = f.readlines()
                        # show last n lines
                        for line in content[-lines:]:
                            line = line.strip()
                            if line:
                                print(f"   {line}")
                except Exception as e:
                    print(f"   error reading log: {e}")


def run_smoke_test(api_url: str):
    """run smoke tests"""
    print("")
    print("running smoke tests...")
    print("-" * 50)
    
    script = load_tests_dir / "smoke_test.py"
    result = subprocess.run(
        [sys.executable, str(script), api_url],
        capture_output=False
    )
    return result.returncode == 0


def run_load_test(api_url: str, homeserver: str, config: dict):
    """run load tests"""
    print("")
    print("running load tests...")
    print("-" * 50)
    
    # build arguments
    args = [
        sys.executable,
        str(load_tests_dir / "agora_test_suite.py"),
        "--api-url", api_url,
        "--homeserver", homeserver
    ]
    
    if config.get('registration_count'):
        args.extend(["--registration-count", str(config['registration_count'])])
    if config.get('server_count'):
        args.extend(["--server-count", str(config['server_count'])])
    if config.get('load_duration'):
        args.extend(["--load-duration", str(config['load_duration'])])
    if config.get('concurrent_users'):
        args.extend(["--concurrent-users", str(config['concurrent_users'])])
    
    args.append("--monitor-docker")
    
    result = subprocess.run(args)
    return result.returncode == 0


def run_chaos_test(api_url: str):
    """run chaos tests"""
    print("")
    print("running chaos tests...")
    print("-" * 50)
    
    script = load_tests_dir / "chaos_test.py"
    result = subprocess.run(
        [sys.executable, str(script), api_url],
        capture_output=False
    )
    return result.returncode == 0


def run_delay_test(api_url: str):
    """run delay/timing tests"""
    print("")
    print("running delay/timing tests...")
    print("-" * 50)
    
    script = load_tests_dir / "delay_test.py"
    result = subprocess.run(
        [sys.executable, str(script), api_url],
        capture_output=False
    )
    return result.returncode == 0


def main():
    parser = argparse.ArgumentParser(description='agora test suite runner')
    parser.add_argument('command', nargs='?', default='smoke',
                       choices=['smoke', 'load', 'chaos', 'delay', 'all'],
                       help='test command to run (default: smoke)')
    parser.add_argument('--api-url', default=os.environ.get('API_URL', default_api_url),
                       help='agora api url')
    parser.add_argument('--homeserver', default=os.environ.get('HOMESERVER', default_homeserver),
                       help='matrix homeserver url')
    
    args = parser.parse_args()
    
    print("agora test suite")
    print("=" * 50)
    print(f"api url: {args.api_url}")
    print(f"homeserver: {args.homeserver}")
    print("")
    
    # check services first
    if not check_services(args.api_url, args.homeserver):
        print("\nservices not ready. please start them first.")
        sys.exit(1)
    
    # load config from environment
    config = {
        'registration_count': os.environ.get('REGISTRATION_COUNT'),
        'server_count': os.environ.get('SERVER_COUNT'),
        'load_duration': os.environ.get('LOAD_DURATION'),
        'concurrent_users': os.environ.get('CONCURRENT_USERS'),
    }
    
    # run requested tests
    success = True
    
    try:
        if args.command == 'smoke':
            success = run_smoke_test(args.api_url)
        elif args.command == 'load':
            success = run_load_test(args.api_url, args.homeserver, config)
        elif args.command == 'chaos':
            success = run_chaos_test(args.api_url)
        elif args.command == 'delay':
            success = run_delay_test(args.api_url)
        elif args.command == 'all':
            success = (run_smoke_test(args.api_url) and
                      run_delay_test(args.api_url) and
                      run_load_test(args.api_url, args.homeserver, config) and
                      run_chaos_test(args.api_url))
    except KeyboardInterrupt:
        print("\ntest interrupted by user")
        success = False
    except Exception as e:
        print(f"\ntest failed with error: {e}")
        import traceback
        traceback.print_exc()
        success = False
    
    print("")
    print("=" * 50)
    if success:
        print("test suite complete - all tests passed")
        sys.exit(0)
    else:
        print("test suite complete - some tests failed")
        sys.exit(1)


if __name__ == '__main__':
    main()
