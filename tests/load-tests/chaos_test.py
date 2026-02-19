#!/usr/bin/env python3
"""
Chaos Test - Deliberately try to break things
"""

import asyncio
import aiohttp
import random
import time
import sys
from typing import List


class ChaosMonkey:
    """Agent of chaos"""
    
    def __init__(self, api_url: str, session: aiohttp.ClientSession):
        self.api_url = api_url
        self.session = session
        self.tokens: List[str] = []
        self.servers: List[str] = []
        self.errors: List[str] = []
        
    async def spam_registrations(self, count: int):
        """rapid-fire registrations"""
        print(f"spamming {count} rapid registrations...")
        
        async def register_one(i: int):
            try:
                async with self.session.post(
                    f"{self.api_url}/auth/register",
                    json={
                        "username": f"chaos_{i}_{int(time.time()*1000)}",
                        "password": "x",
                        "initial_device_display_name": "chaos"
                    },
                    timeout=aiohttp.ClientTimeout(total=5)
                ) as resp:
                    if resp.status == 200:
                        data = await resp.json()
                        self.tokens.append(data.get("access_token"))
                    else:
                        text = await resp.text()
                        if "m_user_in_use" not in text:  # expected for duplicates
                            self.errors.append(f"registration {i}: {resp.status}")
            except Exception as e:
                self.errors.append(f"registration {i}: {e}")
        
        await asyncio.gather(*[register_one(i) for i in range(count)])
        print(f"   created {len(self.tokens)} users, {len(self.errors)} errors")
    
    async def concurrent_server_creation(self):
        """create many servers at once"""
        if not self.tokens:
            print("no tokens for server creation")
            return
            
        print(f"creating servers with {len(self.tokens)} users concurrently...")
        
        async def create_many(token: str, user_idx: int):
            for i in range(5):
                try:
                    async with self.session.post(
                        f"{self.api_url}/rooms/create",
                        json={"access_token": token, "name": f"chaos_server_{user_idx}_{i}", "is_space": True},
                        timeout=aiohttp.ClientTimeout(total=5)
                    ) as resp:
                        if resp.status == 200:
                            data = await resp.json()
                            self.servers.append(data.get("room_id"))
                except Exception as e:
                    self.errors.append(f"server creation: {e}")
        
        await asyncio.gather(*[
            create_many(token, i) for i, token in enumerate(self.tokens[:10])
        ])
        print(f"   created {len(self.servers)} servers")
    
    async def rapid_join_leave(self, iterations: int):
        """join and leave rapidly"""
        if not self.tokens or not self.servers:
            print("no tokens or servers for join/leave")
            return
            
        print(f"rapid join/leave ({iterations} iterations)...")
        
        async def jiggle(token: str):
            for _ in range(iterations):
                server = random.choice(self.servers)
                try:
                    # join
                    async with self.session.post(
                        f"{self.api_url}/rooms/join",
                        json={"access_token": token, "room_id_or_alias": server},
                        timeout=aiohttp.ClientTimeout(total=3)
                    ) as resp:
                        pass  # don't care about result
                    
                    # immediate leave
                    async with self.session.post(
                        f"{self.api_url}/rooms/leave",
                        json={"access_token": token, "room_id": server},
                        timeout=aiohttp.ClientTimeout(total=3)
                    ) as resp:
                        pass
                        
                except Exception:
                    pass  # expected to have some failures
        
        await asyncio.gather(*[
            jiggle(token) for token in self.tokens[:5]
        ])
        print("   join/leave chaos complete")
    
    async def malformed_requests(self):
        """send malformed data"""
        print("sending malformed requests...")
        
        malformations = [
            # missing required fields
            {"url": f"{self.api_url}/auth/register", "json": {}},
            {"url": f"{self.api_url}/rooms/create", "json": {"access_token": "invalid"}},
            # invalid types
            {"url": f"{self.api_url}/rooms/create", "json": {"access_token": 123, "name": None}},
            # empty strings
            {"url": f"{self.api_url}/auth/register", "json": {"username": "", "password": ""}},
            # very long strings
            {"url": f"{self.api_url}/rooms/create", "json": {"access_token": "x"*10000, "name": "test"}},
        ]
        
        for malformed in malformations:
            try:
                async with self.session.post(
                    malformed["url"],
                    json=malformed["json"],
                    timeout=aiohttp.ClientTimeout(total=5)
                ) as resp:
                    # should get 400, not crash
                    if resp.status >= 500:
                        self.errors.append(f"server crash on malformed: {resp.status}")
            except Exception as e:
                self.errors.append(f"exception on malformed: {e}")
        
        print("   malformed request testing complete")
    
    async def race_condition_test(self):
        """try to create race conditions"""
        print("testing race conditions...")
        
        if not self.tokens:
            return
            
        token = self.tokens[0]
        
        # try to create the same channel multiple times simultaneously
        async def create_duplicate(i: int):
            try:
                async with self.session.post(
                    f"{self.api_url}/rooms/create",
                    json={"access_token": token, "name": "race_channel", "is_space": False},
                    timeout=aiohttp.ClientTimeout(total=5)
                ) as resp:
                    return resp.status
            except Exception as e:
                return str(e)
        
        results = await asyncio.gather(*[create_duplicate(i) for i in range(10)])
        print(f"   race results: {set(results)}")


async def run_chaos_tests(api_url: str):
    """run all chaos tests"""
    print("\nagora chaos tests")
    print("="*60)
    print("these tests deliberately try to break things.")
    print("some errors are expected and ok.")
    print("="*60 + "\n")
    
    async with aiohttp.ClientSession() as session:
        monkey = ChaosMonkey(api_url, session)
        
        await monkey.spam_registrations(50)
        await monkey.concurrent_server_creation()
        await monkey.rapid_join_leave(20)
        await monkey.malformed_requests()
        await monkey.race_condition_test()
        
        print("\n" + "="*60)
        print("chaos test summary")
        print("="*60)
        print(f"total users created: {len(monkey.tokens)}")
        print(f"total servers created: {len(monkey.servers)}")
        print(f"errors encountered: {len(monkey.errors)}")
        
        if monkey.errors:
            print("\nsample errors:")
            for err in monkey.errors[:5]:
                print(f"   - {err}")
        
        if len(monkey.errors) < 10:
            print("\nsystem handled chaos reasonably well")
        else:
            print("\nsystem showed stress under chaos")


if __name__ == '__main__':
    api_url = sys.argv[1] if len(sys.argv) > 1 else "http://localhost:3000"
    asyncio.run(run_chaos_tests(api_url))
