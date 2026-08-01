#!/usr/bin/env python3
"""
Neural Symphony - Test Composition Generator
Quick test to verify the system works end-to-end
"""

import asyncio
import sys
import os

# Add backend to path
sys.path.insert(0, os.path.join(os.path.dirname(__file__), 'backend'))

from cerebro_client import CerebroMemoryClient, generate_composition_from_memories

async def test_cerebro_connection():
    """Test if Cerebro MCP is available"""
    print("🧠 Testing Cerebro connection...")
    
    try:
        client = CerebroMemoryClient()
        
        # Try to fetch a few memories
        memories = await client.fetch_memories(
            query="Hermes",
            top_k=5,
            min_salience=0.0
        )
        
        print(f"✅ Cerebro connected! Found {len(memories)} memories")
        
        if memories:
            print("\n📝 Sample memories:")
            for i, m in enumerate(memories[:3]):
                print(f"   {i+1}. [{m.get('type', 'unknown')}] "
                      f"salience: {m.get('salience', 0):.2f}")
                print(f"      Content: {m.get('content', '')[:80]}...")
        
        return memories
        
    except Exception as e:
        print(f"❌ Cerebro connection failed: {e}")
        print("   Will use demo memories instead")
        return None

async def generate_test_composition():
    """Generate a test composition"""
    print("\n🎵 Generating composition...")
    
    # Try to get real memories
    memories = await test_cerebro_connection()
    
    # If no memories, generate demo
    if not memories:
        print("\n📊 Generating demo composition...")
        memories = []
        for i in range(20):
            memories.append({
                "id": f"demo_{i}",
                "type": ["episodic", "procedural", "semantic", "affective"][i % 4],
                "salience": 0.3 + (i / 20) * 0.7,
                "created_at": "2026-08-01T12:00:00Z"
            })
    
    # Generate composition
    composition = generate_composition_from_memories(
        memories,
        title=f"Neural Symphony Test: {len(memories)} Memories",
        bpm=120,
        key="C major"
    )
    
    # Print results
    print(f"\n✅ Composition generated!")
    print(f"   Title: {composition['title']}")
    print(f"   Notes: {composition['note_count']}")
    print(f"   Duration: {composition['duration_seconds']:.1f}s")
    print(f"   Key: {composition['key']} @ {composition['bpm']} BPM")
    
    print(f"\n🎹 Note breakdown by instrument:")
    instrument_counts = {}
    for note in composition['memories']:
        inst = note['instrument']
        instrument_counts[inst] = instrument_counts.get(inst, 0) + 1
    
    for inst, count in sorted(instrument_counts.items()):
        print(f"   {inst:15} {count:3} notes")
    
    print(f"\n🎼 First 10 notes:")
    for i, note in enumerate(composition['memories'][:10]):
        salience_bar = '█' * int(note['salience'] * 10)
        print(f"   {i+1:2}. {note['note']:4} ({note['instrument']:12}) "
              f"vel:{note['velocity']:3} {salience_bar}")
    
    return composition

async def main():
    print("=" * 60)
    print("🎵 Neural Symphony Studio - Test Suite")
    print("=" * 60)
    
    composition = await generate_test_composition()
    
    print("\n" + "=" * 60)
    print("✅ Test complete!")
    print("=" * 60)
    print("\n🚀 To run the full app:")
    print("   ./start.sh")
    print("\n🌐 Then open: http://localhost:5173")
    print("=" * 60)

if __name__ == "__main__":
    asyncio.run(main())