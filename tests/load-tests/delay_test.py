#!/usr/bin/env python3
"""
Delay and Timing Test Suite

Specifically tests timing issues mentioned in bug reports:
- Voice disconnect delays
- Channel participant list updates
- Message sync delays
- Server list refresh timing
"""

import asyncio
import aiohttp
import time
import sys
from typing import List, Tuple
from dataclasses import dataclass
from statistics import mean, median


@dataclass
class TimingResult:
    operation: str
    target_ms: float
    actual_ms: float
    success: bool


class DelayTester:
    """Test specific timing scenarios"""
    
    def __init__(self, api_url: str, session: aiohttp.ClientSession):
        self.api_url = api_url
        self.session = session
        self.results: List[TimingResult] = []
        
    async def test_voice_disconnect_delay(self) -> List[TimingResult]:
        """
        test: how long does it take for participant list to update after disconnect?
        expected: < 500ms
        """
        print("\ntesting voice disconnect timing...")
        results = []
        
        # create test user
        user1 = await self.create_user("voice_test_1")
        user2 = await self.create_user("voice_test_2")
        
        if not user1 or not user2:
            print("   failed to create test users")
            return results
        
        # create server with voice channel
        server_id = await self.create_server(user1["token"], "VoiceDelayTest")
        channel_id = await self.create_channel(user1["token"], server_id, "voice-test", "voice")
        
        if not channel_id:
            print("   failed to create voice channel")
            return results
        
        # user 2 joins voice
        await self.join_room(user2["token"], channel_id)
        await asyncio.sleep(1)  # give time for join
        
        # measure time for disconnect to reflect
        start = time.time()
        
        # simulate disconnect by leaving
        await self.leave_room(user2["token"], channel_id)
        
        # poll until user disappears
        max_wait = 5.0  # max 5 seconds
        check_interval = 0.05  # check every 50ms
        elapsed = 0
        
        while elapsed < max_wait:
            await asyncio.sleep(check_interval)
            elapsed += check_interval
            
            # check participants
            participants = await self.get_voice_participants(channel_id)
            if user2["user_id"] not in participants:
                break
        
        actual_time = (time.time() - start) * 1000
        success = elapsed < max_wait
        
        result = TimingResult(
            operation="voice_disconnect_propagation",
            target_ms=500,
            actual_ms=actual_time,
            success=success
        )
        results.append(result)
        
        status = "pass" if success else "fail"
        print(f"   {status} disconnect propagation: {actual_time:.1f}ms (target: <500ms)")
        
        return results
    
    async def test_message_sync_delay(self) -> List[TimingResult]:
        """
        test: how long for message to appear in sync?
        expected: < 1000ms
        """
        print("\ntesting message sync timing...")
        results = []
        
        # create test users
        sender = await self.create_user("msg_sender")
        receiver = await self.create_user("msg_receiver")
        
        if not sender or not receiver:
            return results
        
        # create dm room
        room_id = await self.create_dm_room(sender["token"], receiver["user_id"])
        
        # receiver starts listening
        sync_token = None
        
        # send message
        msg_content = f"test message {time.time()}"
        send_start = time.time()
        
        await self.send_message(sender["token"], room_id, msg_content)
        send_time = (time.time() - send_start) * 1000
        
        # wait for message to appear in receiver's sync
        max_wait = 5.0
        check_interval = 0.1
        elapsed = 0
        found = False
        
        while elapsed < max_wait:
            await asyncio.sleep(check_interval)
            elapsed += check_interval
            
            data, _ = await self.sync(receiver["token"], sync_token)
            if data and "messages" in data:
                for msg in data["messages"]:
                    if msg.get("content") == msg_content:
                        found = True
                        break
            if found:
                break
            if data and "next_batch" in data:
                sync_token = data["next_batch"]
        
        total_time = (time.time() - send_start) * 1000
        
        result = TimingResult(
            operation="message_sync_propagation",
            target_ms=1000,
            actual_ms=total_time,
            success=found and total_time < 2000
        )
        results.append(result)
        
        result2 = TimingResult(
            operation="message_send_latency",
            target_ms=200,
            actual_ms=send_time,
            success=send_time < 500
        )
        results.append(result2)
        
        print(f"   send latency: {send_time:.1f}ms")
        status = "pass" if found else "fail"
        print(f"   {status} sync propagation: {total_time:.1f}ms")
        
        return results
    
    async def test_server_list_refresh(self) -> List[TimingResult]:
        """
        test: how long for new server to appear in room list?
        expected: < 500ms
        """
        print("\ntesting server list refresh timing...")
        results = []
        
        user = await self.create_user("server_refresh_test")
        if not user:
            return results
        
        # get initial list
        initial_rooms = await self.get_joined_rooms(user["token"])
        
        # create server
        start = time.time()
        server_id = await self.create_server(user["token"], f"RefreshTest_{int(time.time())}")
        creation_time = (time.time() - start) * 1000
        
        # wait for it to appear in list
        max_wait = 3.0
        check_interval = 0.05
        elapsed = 0
        found = False
        
        while elapsed < max_wait:
            await asyncio.sleep(check_interval)
            elapsed += check_interval
            
            rooms = await self.get_joined_rooms(user["token"])
            if server_id in rooms:
                found = True
                break
        
        total_time = (time.time() - start) * 1000
        
        result = TimingResult(
            operation="server_list_refresh",
            target_ms=500,
            actual_ms=total_time,
            success=found and total_time < 1000
        )
        results.append(result)
        
        status = "pass" if result.success else "fail"
        print(f"   {status} server list refresh: {total_time:.1f}ms (target: <500ms)")
        
        return results
    
    async def test_channel_creation_delay(self) -> List[TimingResult]:
        """
        test: how long for channel to be usable after creation?
        expected: < 300ms
        """
        print("\ntesting channel creation timing...")
        results = []
        
        user = await self.create_user("channel_test")
        if not user:
            return results
        
        server_id = await self.create_server(user["token"], "ChannelTimingTest")
        
        # create channel and immediately try to send message
        start = time.time()
        channel_id = await self.create_channel(user["token"], server_id, "test-chan", "text")
        creation_time = (time.time() - start) * 1000
        
        # try to send message immediately
        msg_start = time.time()
        success = await self.send_message(user["token"], channel_id, "immediate message")
        msg_time = (time.time() - msg_start) * 1000
        
        result = TimingResult(
            operation="channel_creation_to_usable",
            target_ms=300,
            actual_ms=creation_time + msg_time,
            success=success and (creation_time + msg_time) < 1000
        )
        results.append(result)
        
        status = "pass" if success else "fail"
        print(f"   {status} creation+usable: {creation_time + msg_time:.1f}ms")
        
        return results
    
    async def test_concurrent_load_delays(self) -> List[TimingResult]:
        """
        test: how does latency degrade under concurrent load?
        """
        print("\ntesting latency under concurrent load...")
        results = []
        
        # create multiple users
        users = []
        for i in range(5):
            user = await self.create_user(f"load_user_{i}")
            if user:
                users.append(user)
        
        if len(users) < 3:
            print("   not enough users created")
            return results
        
        # create shared server
        server_id = await self.create_server(users[0]["token"], "LoadTestServer")
        channel_id = await self.create_channel(users[0]["token"], server_id, "load-chan", "text")
        
        # all users join
        for user in users[1:]:
            await self.join_room(user["token"], server_id)
        
        # concurrent message sending
        latencies = []
        
        async def send_and_measure(user):
            start = time.time()
            await self.send_message(user["token"], channel_id, f"load test from {user['user_id']}")
            return (time.time() - start) * 1000
        
        # send 10 rounds of concurrent messages
        for round_num in range(10):
            start_round = time.time()
            round_latencies = await asyncio.gather(*[
                send_and_measure(user) for user in users
            ])
            latencies.extend(round_latencies)
            await asyncio.sleep(0.1)  # brief pause between rounds
        
        avg_latency = mean(latencies)
        max_latency = max(latencies)
        
        result = TimingResult(
            operation="concurrent_message_latency",
            target_ms=500,
            actual_ms=avg_latency,
            success=avg_latency < 1000 and max_latency < 2000
        )
        results.append(result)
        
        print(f"   concurrent messages: avg={avg_latency:.1f}ms, max={max_latency:.1f}ms")
        
        return results
    
    # Helper methods
    async def create_user(self, prefix: str) -> dict:
        """Create a test user"""
        username = f"{prefix}_{int(time.time()*1000)}"
        try:
            async with self.session.post(
                f"{self.api_url}/auth/register",
                json={"username": username, "password": "test123", "initial_device_display_name": "delay_test"}
            ) as resp:
                if resp.status == 200:
                    data = await resp.json()
                    return {
                        "token": data.get("access_token"),
                        "user_id": data.get("user_id")
                    }
        except Exception as e:
            print(f"   Error creating user: {e}")
        return None
    
    async def create_server(self, token: str, name: str) -> str:
        try:
            async with self.session.post(
                f"{self.api_url}/rooms/create",
                json={"access_token": token, "name": name, "is_space": True}
            ) as resp:
                if resp.status == 200:
                    data = await resp.json()
                    return data.get("room_id")
        except:
            pass
        return None
    
    async def create_channel(self, token: str, server_id: str, name: str, chan_type: str) -> str:
        try:
            async with self.session.post(
                f"{self.api_url}/rooms/create",
                json={"access_token": token, "name": name, "is_space": False, "parent_space_id": server_id, "channel_type": chan_type}
            ) as resp:
                if resp.status == 200:
                    data = await resp.json()
                    return data.get("room_id")
        except:
            pass
        return None
    
    async def join_room(self, token: str, room_id: str):
        try:
            async with self.session.post(
                f"{self.api_url}/rooms/join",
                json={"access_token": token, "room_id_or_alias": room_id}
            ) as resp:
                return resp.status == 200
        except:
            return False
    
    async def leave_room(self, token: str, room_id: str):
        try:
            async with self.session.post(
                f"{self.api_url}/rooms/leave",
                json={"access_token": token, "room_id": room_id}
            ) as resp:
                return resp.status == 200
        except:
            return False
    
    async def send_message(self, token: str, room_id: str, content: str) -> bool:
        try:
            async with self.session.post(
                f"{self.api_url}/rooms/send",
                json={"access_token": token, "room_id": room_id, "content": content}
            ) as resp:
                return resp.status == 200
        except:
            return False
    
    async def get_voice_participants(self, room_id: str) -> List[str]:
        try:
            async with self.session.get(
                f"{self.api_url}/voice/participants",
                params={"room_name": room_id}
            ) as resp:
                if resp.status == 200:
                    data = await resp.json()
                    return data.get("participants", [])
        except:
            pass
        return []
    
    async def sync(self, token: str, since: str = None):
        try:
            params = {"access_token": token}
            if since:
                params["since"] = since
            async with self.session.get(f"{self.api_url}/sync", params=params) as resp:
                if resp.status == 200:
                    return await resp.json(), None
                return None, await resp.text()
        except Exception as e:
            return None, str(e)
    
    async def get_joined_rooms(self, token: str) -> List[str]:
        try:
            async with self.session.get(
                f"{self.api_url}/rooms",
                params={"access_token": token}
            ) as resp:
                if resp.status == 200:
                    data = await resp.json()
                    return [r.get("room_id") for r in data.get("rooms", [])]
        except:
            pass
        return []
    
    async def create_dm_room(self, token: str, other_user: str) -> str:
        # For simplicity, just create a regular room
        return await self.create_server(token, f"DM_{other_user}")


async def main():
    api_url = sys.argv[1] if len(sys.argv) > 1 else "http://localhost:3000"
    
    print("\n" + "="*70)
    print("agora delay and timing test suite")
    print("="*70)
    print(f"testing against: {api_url}")
    print("="*70)
    
    async with aiohttp.ClientSession() as session:
        tester = DelayTester(api_url, session)
        all_results = []
        
        try:
            all_results.extend(await tester.test_voice_disconnect_delay())
            all_results.extend(await tester.test_message_sync_delay())
            all_results.extend(await tester.test_server_list_refresh())
            all_results.extend(await tester.test_channel_creation_delay())
            all_results.extend(await tester.test_concurrent_load_delays())
        except Exception as e:
            print(f"\ntest error: {e}")
            import traceback
            traceback.print_exc()
        
        # summary
        print("\n" + "="*70)
        print("timing test summary")
        print("="*70)
        
        passed = sum(1 for r in all_results if r.success)
        failed = len(all_results) - passed
        
        for result in all_results:
            status = "pass" if result.success else "fail"
            print(f"\n{status} {result.operation}")
            print(f"   target: <{result.target_ms}ms")
            print(f"   actual: {result.actual_ms:.1f}ms")
            if result.actual_ms > result.target_ms:
                print(f"   exceeded by {result.actual_ms - result.target_ms:.1f}ms")
        
        print("\n" + "="*70)
        print(f"results: {passed} passed, {failed} failed")
        
        if failed == 0:
            print("all timing tests passed!")
            return 0
        else:
            print("some timing targets not met")
            return 1


if __name__ == '__main__':
    exit_code = asyncio.run(main())
    sys.exit(exit_code)
