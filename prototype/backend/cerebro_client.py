"""
CerebroCortex-RS Client for Neural Symphony
Real memory integration
"""

import os
import asyncio
from typing import List, Dict, Optional
from mcp import ClientSession, StdioServerParameters
from mcp.client.stdio import stdio_client


class CerebroMemoryClient:
    """Client for fetching memories from CerebroCortex-RS"""
    
    def __init__(self, data_dir: Optional[str] = None):
        self.data_dir = data_dir or os.path.expanduser("~/.cerebro-cortex")
        self.server_params = StdioServerParameters(
            command="~/.local/bin/cerebro-mcp",
            args=[],
            env={"CEREBRO_DATA_DIR": self.data_dir}
        )
    
    async def fetch_memories(
        self,
        agent_id: Optional[str] = None,
        query: str = "",
        top_k: int = 50,
        min_salience: float = 0.3
    ) -> List[Dict]:
        """
        Fetch memories from CerebroCortex
        
        Args:
            agent_id: Filter by specific agent
            query: Semantic search query (empty = all recent)
            top_k: Maximum memories to fetch
            min_salience: Minimum salience threshold
            
        Returns:
            List of memory objects with content, type, salience, etc.
        """
        async with stdio_client(self.server_params) as (read, write):
            async with ClientSession(read, write) as session:
                await session.initialize()
                
                # Build tool arguments
                tool_args = {"query": query, "top_k": top_k}
                if agent_id:
                    tool_args["agent_id"] = agent_id
                
                # Call recall tool
                result = await session.call_tool("recall", tool_args)
                
                # Filter by salience
                memories = result.get("memories", [])
                filtered = [
                    m for m in memories 
                    if m.get("salience", 0) >= min_salience
                ]
                
                return filtered
    
    async def get_memory_neighbors(self, memory_id: str) -> List[Dict]:
        """Get memories directly linked to a given memory"""
        async with stdio_client(self.server_params) as (read, write):
            async with ClientSession(read, write) as session:
                await session.initialize()
                
                result = await session.call_tool(
                    "memory_neighbors",
                    {"memory_id": memory_id}
                )
                
                return result.get("neighbors", [])
    
    async def list_memories_by_type(
        self,
        memory_type: str,
        agent_id: Optional[str] = None,
        limit: int = 50
    ) -> List[Dict]:
        """List memories filtered by type"""
        async with stdio_client(self.server_params) as (read, write):
            async with ClientSession(read, write) as session:
                await session.initialize()
                
                tool_args = {"limit": limit}
                if agent_id:
                    tool_args["agent_id"] = agent_id
                
                result = await session.call_tool(
                    "list_memories",
                    tool_args
                )
                
                # Filter by type
                memories = result.get("memories", [])
                filtered = [m for m in memories if m.get("type") == memory_type]
                
                return filtered[:limit]


# Music mapping utilities
INSTRUMENT_MAP = {
    "episodic": "piano",
    "procedural": "synth_lead",
    "semantic": "strings",
    "affective": "cello",
    "prospective": "flute",
    "schematic": "bass"
}

MEMORY_TYPES = list(INSTRUMENT_MAP.keys())

def map_memory_to_note(memory: Dict, scale: List[str] = None) -> Dict:
    """
    Convert a memory object to musical parameters
    
    Args:
        memory: Memory object from Cerebro
        scale: Musical scale to use (default: C major)
        
    Returns:
        Dictionary with note, velocity, duration, instrument
    """
    if scale is None:
        scale = ["C", "D", "E", "F", "G", "A", "B"]
    
    # Deterministic note selection based on memory ID hash
    note_index = hash(memory["id"]) % len(scale)
    octave = 3 + (hash(memory["id"]) % 3)  # Octaves 3-5
    note = f"{scale[note_index]}{octave}"
    
    # Velocity from salience (40-127 range)
    salience = memory.get("salience", 0.5)
    velocity = int(40 + salience * 87)
    velocity = max(40, min(127, velocity))
    
    # Duration based on salience
    duration = 1.0 + (salience * 2.0)
    
    # Instrument from memory type
    memory_type = memory.get("type", "semantic")
    instrument = INSTRUMENT_MAP.get(memory_type, "piano")
    
    return {
        "memory_id": memory["id"],
        "memory_type": memory_type,
        "salience": salience,
        "note": note,
        "velocity": velocity,
        "duration": duration,
        "instrument": instrument,
        "timestamp": memory.get("created_at", "")
    }


def generate_composition_from_memories(
    memories: List[Dict],
    title: str = "Neural Symphony",
    bpm: int = 120,
    key: str = "C major"
) -> Dict:
    """
    Build a complete composition from a list of memories
    
    Args:
        memories: List of memory objects
        title: Composition title
        bpm: Beats per minute
        key: Musical key
        
    Returns:
        Composition object with all notes and metadata
    """
    from datetime import datetime
    
    notes = [map_memory_to_note(m) for m in memories]
    
    total_duration = sum(n["duration"] for n in notes)
    
    return {
        "title": title,
        "memories": notes,
        "duration_seconds": total_duration,
        "bpm": bpm,
        "key": key,
        "note_count": len(notes),
        "created_at": datetime.utcnow().isoformat() + "Z"
    }


async def demo_cerebro_integration():
    """Demo: Fetch real memories and create composition"""
    client = CerebroMemoryClient()
    
    print("🧠 Fetching memories from CerebroCortex...")
    memories = await client.fetch_memories(
        query="Hermes VR Blender",  # Search for relevant memories
        top_k=30,
        min_salience=0.4
    )
    
    print(f"✅ Found {len(memories)} memories")
    
    composition = generate_composition_from_memories(
        memories,
        title="Neural Symphony: VR & Memory Palace",
        bpm=110,
        key="D minor"
    )
    
    print(f"🎵 Generated composition:")
    print(f"   Title: {composition['title']}")
    print(f"   Notes: {composition['note_count']}")
    print(f"   Duration: {composition['duration_seconds']:.1f}s")
    print(f"   Key: {composition['key']} @ {composition['bpm']} BPM")
    
    print(f"\n🎹 First 5 notes:")
    for i, note in enumerate(composition['memories'][:5]):
        print(f"   {i+1}. {note['note']} ({note['instrument']}) - "
              f"vel:{note['velocity']} dur:{note['duration']:.1f}s "
              f"[{note['memory_type']}]")
    
    return composition


if __name__ == "__main__":
    asyncio.run(demo_cerebro_integration())