"""
Neural Symphony Studio - Backend
Transforms CerebroCortex memories into musical compositions
"""

from fastapi import FastAPI, WebSocket, WebSocketDisconnect
from fastapi.middleware.cors import CORSMiddleware
from fastapi.responses import HTMLResponse
from pydantic import BaseModel
from typing import List, Dict, Optional
import asyncio
import json
import os
from pathlib import Path

app = FastAPI(title="Neural Symphony Studio", version="0.1.0")

# CORS for frontend
app.add_middleware(
    CORSMiddleware,
    allow_origins=["*"],
    allow_credentials=True,
    allow_methods=["*"],
    allow_headers=["*"],
)

# Memory-to-Music mapping configuration
INSTRUMENT_MAP = {
    "episodic": "piano",
    "procedural": "synth_lead",
    "semantic": "strings",
    "affective": "cello",
    "prospective": "flute",
    "schematic": "bass"
}

SALIENCE_VELOCITY_MAP = {
    0.0: 40,  # Very low salience = quiet
    0.3: 60,
    0.5: 80,
    0.7: 100,
    1.0: 127  # Max salience = loud
}

class MemoryNote(BaseModel):
    """Represents a memory as a musical note"""
    memory_id: str
    memory_type: str
    salience: float
    timestamp: str
    note: str  # Musical note (e.g., "C4", "E#5")
    velocity: int  # Volume (0-127)
    duration: float  # Note duration in seconds
    instrument: str

class Composition(BaseModel):
    """A complete musical composition from memories"""
    title: str
    memories: List[MemoryNote]
    duration_seconds: float
    bpm: int
    key: str

# In-memory storage for active compositions
active_compositions: Dict[str, Composition] = {}

@app.get("/")
async def root():
    return {
        "service": "Neural Symphony Studio",
        "status": "running",
        "version": "0.1.0",
        "endpoints": {
            "/compose": "Generate composition from memories",
            "/ws": "WebSocket for real-time streaming",
            "/compositions": "List active compositions"
        }
    }

@app.get("/health")
async def health():
    return {"status": "healthy", "memories_processed": len(active_compositions)}

@app.post("/compose")
async def compose_from_memories(
    agent_id: Optional[str] = None,
    limit: int = 50,
    min_salience: float = 0.3
):
    """
    Generate a musical composition from CerebroCortex memories.
    
    This is the core magic: memories become notes, associations become harmony.
    """
    try:
        # Import Cerebro client
        from cerebro_client import (
            CerebroMemoryClient,
            generate_composition_from_memories
        )
        import asyncio
        
        # Fetch real memories from Cerebro
        client = CerebroMemoryClient()
        
        # Try to fetch memories, fall back to demo if Cerebro not available
        try:
            memories = await client.fetch_memories(
                agent_id=agent_id,
                query="Hermes VR Blender Cerebro",
                top_k=limit,
                min_salience=min_salience
            )
            
            if not memories:
                # No memories found, generate demo
                raise ValueError("No memories found")
                
        except Exception as e:
            print(f"⚠️  Cerebro fetch failed ({e}), using demo memories")
            # Generate demo memories
            memories = []
            base_notes = ["C", "D", "E", "F", "G", "A", "B"]
            octaves = [3, 4, 5]
            
            for i in range(min(limit, 20)):
                memories.append({
                    "id": f"demo_{i}",
                    "type": ["episodic", "procedural", "semantic", "affective"][i % 4],
                    "salience": 0.3 + (i / 20) * 0.7,
                    "created_at": "2026-08-01T12:00:00Z"
                })
        
        # Generate composition
        composition = generate_composition_from_memories(
            memories,
            title=f"Neural Symphony: {len(memories)} Memories",
            bpm=120,
            key="C major"
        )
        
        composition_id = f"comp_{len(active_compositions)}"
        active_compositions[composition_id] = composition
        
        return {
            "composition_id": composition_id,
            "title": composition.title,
            "note_count": composition.note_count,
            "duration": composition.duration_seconds,
            "bpm": composition.bpm,
            "key": composition.key,
            "memories": composition.memories[:10]  # Return first 10 notes
        }
        
    except Exception as e:
        import traceback
        traceback.print_exc()
        return {"error": str(e), "status": "failed"}

@app.websocket("/ws")
async def websocket_endpoint(websocket: WebSocket):
    """
    WebSocket for real-time memory-to-music streaming.
    
    Memories are streamed as they're accessed, creating a live performance.
    """
    await websocket.accept()
    
    try:
        while True:
            # Wait for client to request a stream
            data = await websocket.receive_text()
            request = json.loads(data)
            
            if request.get("action") == "start_stream":
                # Simulate streaming memories as musical events
                for i in range(10):
                    note_event = {
                        "type": "note_on",
                        "memory_type": ["episodic", "procedural", "semantic"][i % 3],
                        "note": f"C{4 + (i % 2)}",
                        "velocity": 60 + (i * 5),
                        "timestamp": asyncio.get_event_loop().time()
                    }
                    await websocket.send_json(note_event)
                    await asyncio.sleep(0.5)
                
                await websocket.send_json({"type": "stream_end"})
            
    except WebSocketDisconnect:
        pass

@app.get("/compositions")
async def list_compositions():
    """List all active compositions"""
    return {
        "compositions": [
            {
                "id": cid,
                "title": comp.title,
                "notes": len(comp.memories),
                "duration": comp.duration_seconds
            }
            for cid, comp in active_compositions.items()
        ]
    }

@app.get("/compositions/{composition_id}")
async def get_composition(composition_id: str):
    """Get details of a specific composition"""
    if composition_id not in active_compositions:
        return {"error": "Composition not found"}
    
    comp = active_compositions[composition_id]
    return {
        "id": composition_id,
        "title": comp.title,
        "memories": [
            {
                "id": m.memory_id,
                "type": m.memory_type,
                "note": m.note,
                "velocity": m.velocity,
                "duration": m.duration,
                "instrument": m.instrument
            }
            for m in comp.memories
        ],
        "duration": comp.duration_seconds,
        "bpm": comp.bpm,
        "key": comp.key
    }

if __name__ == "__main__":
    import uvicorn
    uvicorn.run(app, host="0.0.0.0", port=8765)