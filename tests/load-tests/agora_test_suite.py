#!/usr/bin/env python3
"""
Agora Load Test Suite

Comprehensive testing including:
- Spam operations (join/leave, server creation, messaging)
- Load testing with concurrent users
- Performance monitoring and delay tracking
- Docker log monitoring
- Integration test flows

Usage:
    python3 agora_test_suite.py --help
    python3 agora_test_suite.py --api-url http://localhost:3000 --homeserver http://localhost:8448
"""

import asyncio
import argparse
import aiohttp
import json
import time
import random
import string
import sys
from datetime import datetime
from typing import List, Dict, Optional, Tuple
from dataclasses import dataclass, field
from statistics import mean, median, stdev
import subprocess
import threading
import queue


@dataclass
class TestMetrics:
    """Track metrics for test operations"""
    operation: str
    start_time: float
    end_time: Optional[float] = None
    success: bool = False
    error: Optional[str] = None
    
    @property
    def duration_ms(self) -> float:
        if self.end_time is None:
            return 0.0
        return (self.end_time - self.start_time) * 1000


@dataclass
class TestSession:
    """Track a complete test session"""
    metrics: List[TestMetrics] = field(default_factory=list)
    users_created: List[Dict] = field(default_factory=list)
    servers_created: List[str] = field(default_factory=list)
    messages_sent: int = 0
    errors: List[str] = field(default_factory=list)
    
    def add_metric(self, metric: TestMetrics):
        self.metrics.append(metric)
    
    def get_stats(self, operation_type: str) -> Dict:
        """Get statistics for a specific operation type"""
        relevant = [m for m in self.metrics if m.operation == operation_type and m.end_time is not None]
        if not relevant:
            return {"count": 0, "avg_ms": 0, "median_ms": 0, "min_ms": 0, "max_ms": 0, "success_rate": 0}
        
        durations = [m.duration_ms for m in relevant]
        successful = [m for m in relevant if m.success]
        
        return {
            "count": len(relevant),
            "avg_ms": mean(durations),
            "median_ms": median(durations),
            "min_ms": min(durations),
            "max_ms": max(durations),
            "stddev_ms": stdev(durations) if len(durations) > 1 else 0,
            "success_rate": len(successful) / len(relevant) * 100,
            "total_errors": len(relevant) - len(successful)
        }


class AgoraClient:
    """Client for interacting with Agora API"""
    
    def __init__(self, api_url: str, homeserver_url: str, session: aiohttp.ClientSession):
        self.api_url = api_url.rstrip('/')
        self.homeserver_url = homeserver_url.rstrip('/')
        self.session = session
        self.access_token: Optional[str] = None
        self.user_id: Optional[str] = None
        self.device_id: Optional[str] = None
        
    async def register(self, username: str, password: str) -> Tuple[bool, Optional[str]]:
        """Register a new user"""
        url = f"{self.api_url}/auth/register"
        payload = {
            "username": username,
            "password": password,
            "initial_device_display_name": f"test_device_{username}"
        }
        
        try:
            async with self.session.post(url, json=payload) as resp:
                if resp.status == 200:
                    data = await resp.json()
                    self.access_token = data.get("access_token")
                    self.user_id = data.get("user_id")
                    self.device_id = data.get("device_id")
                    return True, None
                else:
                    text = await resp.text()
                    return False, f"HTTP {resp.status}: {text}"
        except Exception as e:
            return False, str(e)
    
    async def login(self, username: str, password: str) -> Tuple[bool, Optional[str]]:
        """Login existing user"""
        url = f"{self.api_url}/auth/login"
        payload = {
            "username": username,
            "password": password,
            "initial_device_display_name": f"test_device_{username}"
        }
        
        try:
            async with self.session.post(url, json=payload) as resp:
                if resp.status == 200:
                    data = await resp.json()
                    self.access_token = data.get("access_token")
                    self.user_id = data.get("user_id")
                    self.device_id = data.get("device_id")
                    return True, None
                else:
                    text = await resp.text()
                    return False, f"HTTP {resp.status}: {text}"
        except Exception as e:
            return False, str(e)
    
    async def create_server(self, name: str) -> Tuple[Optional[str], Optional[str]]:
        """Create a new server (space)"""
        if not self.access_token:
            return None, "Not authenticated"
            
        url = f"{self.api_url}/rooms/create"
        payload = {
            "access_token": self.access_token,
            "name": name,
            "is_space": True
        }
        
        try:
            async with self.session.post(url, json=payload) as resp:
                if resp.status == 200:
                    data = await resp.json()
                    return data.get("room_id"), None
                else:
                    text = await resp.text()
                    return None, f"HTTP {resp.status}: {text}"
        except Exception as e:
            return None, str(e)
    
    async def leave_server(self, room_id: str) -> Tuple[bool, Optional[str]]:
        """Leave a server"""
        if not self.access_token:
            return False, "Not authenticated"
            
        url = f"{self.api_url}/rooms/leave"
        payload = {
            "access_token": self.access_token,
            "room_id": room_id
        }
        
        try:
            async with self.session.post(url, json=payload) as resp:
                if resp.status == 200:
                    return True, None
                else:
                    text = await resp.text()
                    return False, f"HTTP {resp.status}: {text}"
        except Exception as e:
            return False, str(e)
    
    async def create_channel(self, name: str, server_id: str, channel_type: str = "text") -> Tuple[Optional[str], Optional[str]]:
        """Create a channel in a server"""
        if not self.access_token:
            return None, "Not authenticated"
            
        url = f"{self.api_url}/rooms/create"
        payload = {
            "access_token": self.access_token,
            "name": name,
            "is_space": False,
            "parent_space_id": server_id,
            "channel_type": channel_type
        }
        
        try:
            async with self.session.post(url, json=payload) as resp:
                if resp.status == 200:
                    data = await resp.json()
                    return data.get("room_id"), None
                else:
                    text = await resp.text()
                    return None, f"HTTP {resp.status}: {text}"
        except Exception as e:
            return None, str(e)
    
    async def send_message(self, room_id: str, content: str) -> Tuple[bool, Optional[str]]:
        """Send a message to a room"""
        if not self.access_token:
            return False, "Not authenticated"
            
        url = f"{self.api_url}/rooms/send"
        payload = {
            "access_token": self.access_token,
            "room_id": room_id,
            "content": content
        }
        
        try:
            async with self.session.post(url, json=payload) as resp:
                if resp.status == 200:
                    return True, None
                else:
                    text = await resp.text()
                    return False, f"HTTP {resp.status}: {text}"
        except Exception as e:
            return False, str(e)
    
    async def get_room_members(self, room_id: str) -> Tuple[Optional[List], Optional[str]]:
        """Get room members"""
        if not self.access_token:
            return None, "Not authenticated"
            
        url = f"{self.api_url}/rooms/members"
        params = {"access_token": self.access_token, "room_id": room_id}
        
        try:
            async with self.session.get(url, params=params) as resp:
                if resp.status == 200:
                    data = await resp.json()
                    return data.get("members", []), None
                else:
                    text = await resp.text()
                    return None, f"HTTP {resp.status}: {text}"
        except Exception as e:
            return None, str(e)
    
    async def sync(self, since: Optional[str] = None) -> Tuple[Optional[Dict], Optional[str]]:
        """Sync messages"""
        if not self.access_token:
            return None, "Not authenticated"
            
        url = f"{self.api_url}/sync"
        params = {"access_token": self.access_token}
        if since:
            params["since"] = since
            
        try:
            async with self.session.get(url, params=params) as resp:
                if resp.status == 200:
                    return await resp.json(), None
                else:
                    text = await resp.text()
                    return None, f"HTTP {resp.status}: {text}"
        except Exception as e:
            return None, str(e)


class LoadTester:
    """Main load testing orchestrator"""
    
    def __init__(self, api_url: str, homeserver_url: str, session: TestSession):
        self.api_url = api_url
        self.homeserver_url = homeserver_url
        self.session = session
        self.clients: List[AgoraClient] = []
        
    async def run_test_suite(self, config: Dict):
        """Run the complete test suite"""
        print("\n" + "="*70)
        print(f"AGORA LOAD TEST SUITE - {datetime.now()}")
        print("="*70)
        print(f"API URL: {self.api_url}")
        print(f"Homeserver: {self.homeserver_url}")
        print(f"Config: {json.dumps(config, indent=2)}")
        print("="*70 + "\n")
        
        async with aiohttp.ClientSession() as http_session:
            # Test 1: User Registration Stress Test
            if config.get('test_registration', True):
                await self.test_registration_spam(
                    http_session,
                    count=config.get('registration_count', 10),
                    concurrent=config.get('concurrent_users', 5)
                )
            
            # Test 2: Server Creation Spam
            if config.get('test_server_creation', True):
                await self.test_server_creation_spam(
                    http_session,
                    count=config.get('server_creation_count', 20),
                    concurrent=config.get('concurrent_servers', 3)
                )
            
            # Test 3: Join/Leave Spam
            if config.get('test_join_leave', True):
                await self.test_join_leave_spam(
                    http_session,
                    iterations=config.get('join_leave_iterations', 50),
                    concurrent=config.get('concurrent_joiners', 5)
                )
            
            # Test 4: Message Spam
            if config.get('test_messaging', True):
                await self.test_message_spam(
                    http_session,
                    messages_per_room=config.get('messages_per_room', 100),
                    rooms=config.get('message_rooms', 3)
                )
            
            # Test 5: Mixed Load Test
            if config.get('test_mixed_load', True):
                await self.test_mixed_load(
                    http_session,
                    duration_seconds=config.get('load_test_duration', 60),
                    concurrent_users=config.get('concurrent_load_users', 10)
                )
        
        # Print results
        self.print_results()
    
    async def test_registration_spam(self, http_session: aiohttp.ClientSession, count: int, concurrent: int):
        """Spam user registrations"""
        print(f"\n📊 TEST 1: Registration Spam ({count} users, {concurrent} concurrent)...")
        
        semaphore = asyncio.Semaphore(concurrent)
        
        async def register_one(index: int):
            async with semaphore:
                username = f"loadtest_{int(time.time())}_{index}_{random.randint(1000, 9999)}"
                password = "testpassword123"
                
                metric = TestMetrics(operation="registration", start_time=time.time())
                client = AgoraClient(self.api_url, self.homeserver_url, http_session)
                
                try:
                    success, error = await client.register(username, password)
                    metric.success = success
                    if error:
                        metric.error = error
                    
                    if success:
                        self.session.users_created.append({
                            "username": username,
                            "user_id": client.user_id,
                            "access_token": client.access_token
                        })
                        self.clients.append(client)
                except Exception as e:
                    metric.error = str(e)
                finally:
                    metric.end_time = time.time()
                    self.session.add_metric(metric)
        
        tasks = [register_one(i) for i in range(count)]
        await asyncio.gather(*tasks, return_exceptions=True)
        
        stats = self.session.get_stats("registration")
        print(f"   ✅ Completed: {stats['count']} registrations")
        print(f"   ⏱️  Avg latency: {stats['avg_ms']:.2f}ms")
        print(f"   📈 Success rate: {stats['success_rate']:.1f}%")
        print(f"   ❌ Errors: {stats['total_errors']}")
    
    async def test_server_creation_spam(self, http_session: aiohttp.ClientSession, count: int, concurrent: int):
        """Spam server creation"""
        print(f"\n📊 TEST 2: Server Creation Spam ({count} servers, {concurrent} concurrent)...")
        
        # Create a test user for server creation
        client = AgoraClient(self.api_url, self.homeserver_url, http_session)
        username = f"server_spammer_{int(time.time())}"
        success, _ = await client.register(username, "testpass123")
        
        if not success:
            print("   ❌ Failed to create test user")
            return
        
        semaphore = asyncio.Semaphore(concurrent)
        
        async def create_one(index: int):
            async with semaphore:
                server_name = f"SpamServer_{index}_{random.randint(1000, 9999)}"
                metric = TestMetrics(operation="server_creation", start_time=time.time())
                
                try:
                    room_id, error = await client.create_server(server_name)
                    metric.success = room_id is not None
                    if error:
                        metric.error = error
                    if room_id:
                        self.session.servers_created.append(room_id)
                except Exception as e:
                    metric.error = str(e)
                finally:
                    metric.end_time = time.time()
                    self.session.add_metric(metric)
        
        tasks = [create_one(i) for i in range(count)]
        await asyncio.gather(*tasks, return_exceptions=True)
        
        stats = self.session.get_stats("server_creation")
        print(f"   ✅ Created: {stats['count']} servers")
        print(f"   ⏱️  Avg latency: {stats['avg_ms']:.2f}ms")
        print(f"   📈 Success rate: {stats['success_rate']:.1f}%")
    
    async def test_join_leave_spam(self, http_session: aiohttp.ClientSession, iterations: int, concurrent: int):
        """Spam join and leave operations"""
        print(f"\n📊 TEST 3: Join/Leave Spam ({iterations} iterations, {concurrent} concurrent)...")
        
        if len(self.session.servers_created) < 3:
            # Create some servers to join/leave
            client = AgoraClient(self.api_url, self.homeserver_url, http_session)
            await client.register(f"jl_test_{int(time.time())}", "testpass123")
            for i in range(5):
                room_id, _ = await client.create_server(f"JLTarget_{i}")
                if room_id:
                    self.session.servers_created.append(room_id)
        
        # Create users for join/leave
        users = []
        for i in range(min(concurrent * 2, 20)):
            client = AgoraClient(self.api_url, self.homeserver_url, http_session)
            success, _ = await client.register(f"jl_user_{i}_{int(time.time())}", "testpass123")
            if success:
                users.append(client)
        
        semaphore = asyncio.Semaphore(concurrent)
        
        async def join_leave_once(user_index: int, iteration: int):
            async with semaphore:
                user = users[user_index % len(users)]
                server_id = random.choice(self.session.servers_created)
                
                # Join
                metric_join = TestMetrics(operation="join", start_time=time.time())
                try:
                    # Use join endpoint through invite flow
                    url = f"{self.api_url}/rooms/join"
                    payload = {
                        "access_token": user.access_token,
                        "room_id_or_alias": server_id
                    }
                    async with http_session.post(url, json=payload) as resp:
                        metric_join.success = resp.status == 200
                        if resp.status != 200:
                            metric_join.error = await resp.text()
                except Exception as e:
                    metric_join.error = str(e)
                finally:
                    metric_join.end_time = time.time()
                    self.session.add_metric(metric_join)
                
                # Small delay to simulate real usage
                await asyncio.sleep(random.uniform(0.1, 0.5))
                
                # Leave
                metric_leave = TestMetrics(operation="leave", start_time=time.time())
                try:
                    success, error = await user.leave_server(server_id)
                    metric_leave.success = success
                    if error:
                        metric_leave.error = error
                except Exception as e:
                    metric_leave.error = str(e)
                finally:
                    metric_leave.end_time = time.time()
                    self.session.add_metric(metric_leave)
        
        tasks = [join_leave_once(i % len(users), i) for i in range(iterations)]
        await asyncio.gather(*tasks, return_exceptions=True)
        
        join_stats = self.session.get_stats("join")
        leave_stats = self.session.get_stats("leave")
        print(f"   ✅ Join ops: {join_stats['count']} (avg {join_stats['avg_ms']:.2f}ms)")
        print(f"   ✅ Leave ops: {leave_stats['count']} (avg {leave_stats['avg_ms']:.2f}ms)")
        print(f"   📈 Join success: {join_stats['success_rate']:.1f}%")
        print(f"   📈 Leave success: {leave_stats['success_rate']:.1f}%")
    
    async def test_message_spam(self, http_session: aiohttp.ClientSession, messages_per_room: int, rooms: int):
        """Spam messages in rooms"""
        print(f"\n📊 TEST 4: Message Spam ({messages_per_room} msgs × {rooms} rooms)...")
        
        # Create test servers with text channels
        client = AgoraClient(self.api_url, self.homeserver_url, http_session)
        await client.register(f"msg_spammer_{int(time.time())}", "testpass123")
        
        text_channels = []
        for i in range(rooms):
            server_id, _ = await client.create_server(f"MsgServer_{i}")
            if server_id:
                channel_id, _ = await client.create_channel("spam-channel", server_id, "text")
                if channel_id:
                    text_channels.append(channel_id)
        
        if not text_channels:
            print("   ❌ Failed to create test channels")
            return
        
        print(f"   Created {len(text_channels)} test channels")
        
        async def spam_channel(channel_id: str):
            for msg_idx in range(messages_per_room):
                metric = TestMetrics(operation="message_send", start_time=time.time())
                
                try:
                    content = f"Test message {msg_idx}: {''.join(random.choices(string.ascii_letters, k=50))}"
                    success, error = await client.send_message(channel_id, content)
                    metric.success = success
                    if error:
                        metric.error = error
                    if success:
                        self.session.messages_sent += 1
                except Exception as e:
                    metric.error = str(e)
                finally:
                    metric.end_time = time.time()
                    self.session.add_metric(metric)
                
                # Small delay to avoid overwhelming
                await asyncio.sleep(0.01)
        
        await asyncio.gather(*[spam_channel(ch) for ch in text_channels])
        
        stats = self.session.get_stats("message_send")
        print(f"   ✅ Sent: {self.session.messages_sent} messages")
        print(f"   ⏱️  Avg latency: {stats['avg_ms']:.2f}ms")
        print(f"   📈 Success rate: {stats['success_rate']:.1f}%")
        print(f"   🚀 Throughput: {self.session.messages_sent / (stats['avg_ms'] * stats['count'] / 1000):.1f} msg/sec")
    
    async def test_mixed_load(self, http_session: aiohttp.ClientSession, duration_seconds: int, concurrent_users: int):
        """Run mixed load for a duration"""
        print(f"\n📊 TEST 5: Mixed Load Test ({duration_seconds}s with {concurrent_users} concurrent users)...")
        
        # Setup: Create servers and users
        test_servers = []
        test_users = []
        
        # Create servers
        setup_client = AgoraClient(self.api_url, self.homeserver_url, http_session)
        await setup_client.register(f"mixed_setup_{int(time.time())}", "testpass123")
        for i in range(5):
            sid, _ = await setup_client.create_server(f"MixedLoad_{i}")
            if sid:
                cid, _ = await setup_client.create_channel("general", sid, "text")
                test_servers.append({"server": sid, "channel": cid})
        
        # Create users
        for i in range(concurrent_users):
            client = AgoraClient(self.api_url, self.homeserver_url, http_session)
            success, _ = await client.register(f"mixed_user_{i}_{int(time.time())}", "testpass123")
            if success:
                # Join a random server
                server = random.choice(test_servers)
                url = f"{self.api_url}/rooms/join"
                payload = {"access_token": client.access_token, "room_id_or_alias": server["server"]}
                async with http_session.post(url, json=payload) as resp:
                    if resp.status == 200:
                        test_users.append({"client": client, "server": server})
        
        print(f"   Setup: {len(test_users)} users in {len(test_servers)} servers")
        
        start_time = time.time()
        operation_count = 0
        
        async def random_operation(user_data: Dict):
            nonlocal operation_count
            client = user_data["client"]
            server = user_data["server"]
            
            while time.time() - start_time < duration_seconds:
                op = random.choice(["message", "sync", "members"])
                metric = TestMetrics(operation=f"mixed_{op}", start_time=time.time())
                
                try:
                    if op == "message":
                        success, _ = await client.send_message(
                            server["channel"], 
                            f"Load test msg {operation_count}"
                        )
                        metric.success = success
                    elif op == "sync":
                        data, _ = await client.sync()
                        metric.success = data is not None
                    elif op == "members":
                        members, _ = await client.get_room_members(server["channel"])
                        metric.success = members is not None
                    
                    operation_count += 1
                except Exception as e:
                    metric.error = str(e)
                finally:
                    metric.end_time = time.time()
                    self.session.add_metric(metric)
                
                await asyncio.sleep(random.uniform(0.1, 1.0))
        
        await asyncio.gather(*[random_operation(u) for u in test_users])
        
        total_time = time.time() - start_time
        print(f"   ✅ Completed {operation_count} operations in {total_time:.1f}s")
        print(f"   🚀 Throughput: {operation_count/total_time:.1f} ops/sec")
    
    def print_results(self):
        """Print final test results"""
        print("\n" + "="*70)
        print("TEST RESULTS SUMMARY")
        print("="*70)
        
        operations = ["registration", "server_creation", "join", "leave", "message_send", "mixed_message", "mixed_sync", "mixed_members"]
        
        for op in operations:
            stats = self.session.get_stats(op)
            if stats['count'] > 0:
                print(f"\n📊 {op.upper().replace('_', ' ')}")
                print(f"   Count: {stats['count']}")
                print(f"   Success: {stats['success_rate']:.1f}% ({stats['count'] - stats['total_errors']}/{stats['count']})")
                print(f"   Latency: avg={stats['avg_ms']:.2f}ms, median={stats['median_ms']:.2f}ms")
                print(f"            min={stats['min_ms']:.2f}ms, max={stats['max_ms']:.2f}ms")
                if stats['stddev_ms'] > 0:
                    print(f"   StdDev: {stats['stddev_ms']:.2f}ms")
        
        print("\n" + "="*70)
        print(f"✅ Test suite completed at {datetime.now()}")
        print("="*70)


class DockerLogMonitor:
    """Monitor Docker logs in real-time"""
    
    def __init__(self, services: List[str]):
        self.services = services
        self.running = False
        self.threads = []
        self.log_queue = queue.Queue()
        
    def start(self):
        """Start monitoring logs"""
        self.running = True
        for service in self.services:
            t = threading.Thread(target=self._monitor_service, args=(service,))
            t.daemon = True
            t.start()
            self.threads.append(t)
        print(f"\n🐳 Started monitoring Docker logs for: {', '.join(self.services)}\n")
    
    def stop(self):
        """Stop monitoring"""
        self.running = False
        for t in self.threads:
            t.join(timeout=2)
    
    def _monitor_service(self, service: str):
        """Monitor a single service"""
        try:
            process = subprocess.Popen(
                ["docker", "logs", "-f", f"--tail=10", f"agora_{service}" if not service.startswith("agora_") else service],
                stdout=subprocess.PIPE,
                stderr=subprocess.STDOUT,
                universal_newlines=True
            )
            
            while self.running:
                line = process.stdout.readline()
                if line:
                    self.log_queue.put((service, line.strip()))
                else:
                    break
        except Exception as e:
            self.log_queue.put((service, f"ERROR: {e}"))
    
    def print_recent_logs(self, lines: int = 50):
        """Print recent logs from queue"""
        print(f"\n📋 Recent Docker Logs (last {lines} lines per service):")
        print("-"*70)
        
        # Group logs by service
        logs_by_service: Dict[str, List[str]] = {s: [] for s in self.services}
        
        # Drain queue
        while not self.log_queue.empty():
            service, line = self.log_queue.get()
            if service in logs_by_service:
                logs_by_service[service].append(line)
        
        # Print grouped
        for service, logs in logs_by_service.items():
            print(f"\n📝 {service.upper()} (showing last {min(lines, len(logs))} of {len(logs)}):")
            for line in logs[-lines:]:
                print(f"   {line}")


async def main():
    parser = argparse.ArgumentParser(description='Agora Load Test Suite')
    parser.add_argument('--api-url', default='http://localhost:3000', help='Agora API URL')
    parser.add_argument('--homeserver', default='http://localhost:8448', help='Matrix homeserver URL')
    parser.add_argument('--registration-count', type=int, default=10, help='Number of registrations to test')
    parser.add_argument('--server-count', type=int, default=20, help='Number of servers to create')
    parser.add_argument('--join-leave-iterations', type=int, default=50, help='Join/leave iterations')
    parser.add_argument('--message-count', type=int, default=100, help='Messages per room')
    parser.add_argument('--load-duration', type=int, default=60, help='Mixed load test duration (seconds)')
    parser.add_argument('--concurrent-users', type=int, default=10, help='Concurrent users for load test')
    parser.add_argument('--monitor-docker', action='store_true', help='Monitor Docker logs during tests')
    parser.add_argument('--skip', nargs='+', choices=['registration', 'servers', 'joinleave', 'messages', 'mixed'], 
                       help='Skip specific tests')
    
    args = parser.parse_args()
    
    # Configuration
    config = {
        'test_registration': 'registration' not in (args.skip or []),
        'test_server_creation': 'servers' not in (args.skip or []),
        'test_join_leave': 'joinleave' not in (args.skip or []),
        'test_messaging': 'messages' not in (args.skip or []),
        'test_mixed_load': 'mixed' not in (args.skip or []),
        'registration_count': args.registration_count,
        'server_creation_count': args.server_count,
        'join_leave_iterations': args.join_leave_iterations,
        'messages_per_room': args.message_count,
        'message_rooms': 3,
        'load_test_duration': args.load_duration,
        'concurrent_load_users': args.concurrent_users,
    }
    
    # Start Docker monitoring if requested
    docker_monitor = None
    if args.monitor_docker:
        docker_monitor = DockerLogMonitor(['conduit', 'agora_livekit', 'api'])
        docker_monitor.start()
    
    try:
        # Run tests
        session = TestSession()
        tester = LoadTester(args.api_url, args.homeserver, session)
        await tester.run_test_suite(config)
        
        # Print Docker logs if monitoring
        if docker_monitor:
            docker_monitor.print_recent_logs()
        
    except KeyboardInterrupt:
        print("\n\n⚠️  Test interrupted by user")
    except Exception as e:
        print(f"\n\n❌ Test failed with error: {e}")
        import traceback
        traceback.print_exc()
    finally:
        if docker_monitor:
            docker_monitor.stop()


if __name__ == '__main__':
    asyncio.run(main())
