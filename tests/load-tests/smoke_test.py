#!/usr/bin/env python3
"""
Simple smoke test - quick validation that the system is working
"""

import asyncio
import aiohttp
import sys
import time


async def smoke_test(api_url: str = "http://localhost:3000"):
    """Run basic smoke tests"""
    print("🧪 AGORA SMOKE TEST")
    print("="*50)
    
    async with aiohttp.ClientSession() as session:
        tests_passed = 0
        tests_failed = 0
        
        # Test 1: Health check
        print("\n1️⃣ Testing API health...")
        try:
            async with session.get(f"{api_url}/health") as resp:
                if resp.status == 200:
                    print("   ✅ API is healthy")
                    tests_passed += 1
                else:
                    print(f"   ❌ API returned {resp.status}")
                    tests_failed += 1
        except Exception as e:
            print(f"   ❌ API connection failed: {e}")
            tests_failed += 1
        
        # Test 2: User registration
        print("\n2️⃣ Testing user registration...")
        username = f"smoke_test_{int(time.time())}"
        try:
            async with session.post(
                f"{api_url}/auth/register",
                json={"username": username, "password": "smoke123", "initial_device_display_name": "smoke"}
            ) as resp:
                if resp.status == 200:
                    data = await resp.json()
                    access_token = data.get("access_token")
                    user_id = data.get("user_id")
                    print(f"   ✅ Registered user: {user_id}")
                    tests_passed += 1
                    
                    # Test 3: Server creation
                    print("\n3️⃣ Testing server creation...")
                    async with session.post(
                        f"{api_url}/rooms/create",
                        json={"access_token": access_token, "name": "SmokeServer", "is_space": True}
                    ) as resp2:
                        if resp2.status == 200:
                            server_data = await resp2.json()
                            server_id = server_data.get("room_id")
                            print(f"   ✅ Created server: {server_id}")
                            tests_passed += 1
                            
                            # Test 4: Channel creation
                            print("\n4️⃣ Testing channel creation...")
                            async with session.post(
                                f"{api_url}/rooms/create",
                                json={
                                    "access_token": access_token, 
                                    "name": "smoke-channel", 
                                    "is_space": False,
                                    "parent_space_id": server_id,
                                    "channel_type": "text"
                                }
                            ) as resp3:
                                if resp3.status == 200:
                                    print("   ✅ Created text channel")
                                    tests_passed += 1
                                    
                                    # Test 5: Send message
                                    print("\n5️⃣ Testing message sending...")
                                    channel_data = await resp3.json()
                                    channel_id = channel_data.get("room_id")
                                    async with session.post(
                                        f"{api_url}/rooms/send",
                                        json={
                                            "access_token": access_token,
                                            "room_id": channel_id,
                                            "content": "Smoke test message"
                                        }
                                    ) as resp4:
                                        if resp4.status == 200:
                                            print("   ✅ Sent message")
                                            tests_passed += 1
                                        else:
                                            print(f"   ❌ Message failed: {resp4.status}")
                                            tests_failed += 1
                                else:
                                    print(f"   ❌ Channel creation failed: {resp3.status}")
                                    tests_failed += 1
                        else:
                            print(f"   ❌ Server creation failed: {resp2.status}")
                            tests_failed += 1
                else:
                    print(f"   ❌ Registration failed: {resp.status}")
                    tests_failed += 1
        except Exception as e:
            print(f"   ❌ Registration error: {e}")
            tests_failed += 1
        
        # Test 6: Sync
        print("\n6️⃣ Testing sync endpoint...")
        if 'access_token' in dir() or 'access_token' in locals():
            try:
                async with session.get(
                    f"{api_url}/sync",
                    params={"access_token": access_token}
                ) as resp:
                    if resp.status == 200:
                        print("   ✅ Sync working")
                        tests_passed += 1
                    else:
                        print(f"   ❌ Sync failed: {resp.status}")
                        tests_failed += 1
            except Exception as e:
                print(f"   ❌ Sync error: {e}")
                tests_failed += 1
        else:
            print("   ⏭️  Skipped (no access token)")
    
    # Results
    print("\n" + "="*50)
    print(f"RESULTS: {tests_passed} passed, {tests_failed} failed")
    if tests_failed == 0:
        print("✅ All smoke tests passed!")
        return 0
    else:
        print("❌ Some tests failed")
        return 1


if __name__ == '__main__':
    api_url = sys.argv[1] if len(sys.argv) > 1 else "http://localhost:3000"
    result = asyncio.run(smoke_test(api_url))
    sys.exit(result)
