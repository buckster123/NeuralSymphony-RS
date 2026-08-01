# Neural Symphony Studio - Architecture

## Overview
Neural Symphony Studio transforms CerebroCortex memories into an interactive audio-visual experience. Memories become musical notes, associations create harmony, and salience controls dynamics.

## System Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                    Frontend (Vue 3 + Three.js)              │
│  ┌─────────────┐  ┌──────────────┐  ┌───────────────────┐  │
│  │  3D Viz     │  │  Web Audio   │  │  Control Panel    │  │
│  │  Canvas     │  │  Engine      │  │  (Stats/Controls) │  │
│  └─────────────┘  └──────────────┘  └───────────────────┘  │
└──────────────────────────┬──────────────────────────────────┘
                           │ WebSocket + REST API
┌──────────────────────────▼──────────────────────────────────┐
│                  Backend (FastAPI)                          │
│  ┌──────────────┐  ┌──────────────┐  ┌─────────────────┐   │
│  │ Memory       │  │ Music        │  │ Suno AI         │   │
│  │ Extractor    │  │ Mapper       │  │ Integration     │   │
│  └──────────────┘  └──────────────┘  └─────────────────┘   │
└──────────────────────────┬──────────────────────────────────┘
                           │ MCP Protocol
┌──────────────────────────▼──────────────────────────────────┐
│              CerebroCortex-RS (Memory Layer)                │
│  ┌──────────────┐  ┌──────────────┐  ┌─────────────────┐   │
│  │ SQLite       │  │ sqlite-vec   │  │ petgraph        │   │
│  │ Storage      │  │ (Embeddings) │  │ (Associations)  │   │
│  └──────────────┘  └──────────────┘  └─────────────────┘   │
└─────────────────────────────────────────────────────────────┘
```

## Core Components

### 1. Memory Extractor
- **Purpose:** Fetch memories from CerebroCortex
- **Methods:**
  - `recall(query, top_k)` - Semantic search
  - `list_memories(agent_id, type)` - Filtered listing
  - `memory_neighbors(memory_id)` - Association graph
- **Output:** Structured memory objects with metadata

### 2. Music Mapper
- **Purpose:** Transform memory attributes into musical parameters
- **Mappings:**
  - `memory_type` → `instrument` (piano, strings, synth, etc.)
  - `salience` → `velocity` (volume 0-127)
  - `timestamp` → `temporal position` (song structure)
  - `associations` → `harmony/chords`
- **Algorithm:** 
  ```python
  note = scale_note(memory_id)  # Deterministic based on ID hash
  velocity = map_salience(memory.salience)
  duration = 1.0 + (salience * 2)  # Important memories = longer notes
  ```

### 3. Audio Engine
- **Real-time:** Web Audio API for browser playback
- **Generated:** Suno AI for full compositions
- **Features:**
  - Polyphonic playback (multiple instruments)
  - Dynamic mixing (salience-based volume)
  - Spatial audio (3D position = panning)

### 4. 3D Visualization
- **Technology:** Three.js WebGL
- **Representation:**
  - Each memory = glowing sphere
  - Color = memory type
  - Size = salience
  - Position = association cluster
  - Pulse = playing state
- **Camera:** Orbital, auto-rotate, zoom/pan controls

## Data Flow

1. **User clicks "Generate Composition"**
2. Frontend sends POST `/api/compose?limit=50&min_salience=0.3`
3. Backend:
   - Connects to Cerebro MCP
   - Fetches memories via `recall()` or `list_memories()`
   - Maps each memory to musical note
   - Builds composition object
4. Backend returns composition metadata
5. Frontend:
   - Updates 3D visualization (node sizes/colors)
   - Starts Web Audio playback
   - Shows playing indicator
6. As notes play, corresponding 3D nodes pulse
7. After duration, playback ends, visualization settles

## Memory-to-Music Mapping Table

| Memory Type   | Instrument   | Color     | Range      |
|---------------|--------------|-----------|------------|
| episodic      | Piano        | #FF6B6B   | C3-G5      |
| procedural    | Synth Lead   | #4ECDC4   | D4-A5      |
| semantic      | Strings      | #45B7D1   | E3-F#5     |
| affective     | Cello        | #96CEB4   | C2-D4      |
| prospective   | Flute        | #FFEAA7   | F4-C6      |
| schematic     | Bass         | #DDA0DD   | C1-G3      |

## Salience Dynamics

| Salience | Velocity | Duration | Visual Size |
|----------|----------|----------|-------------|
| 0.0-0.3  | 40-60    | 1.0s     | 0.8x        |
| 0.3-0.5  | 60-80    | 1.5s     | 1.0x        |
| 0.5-0.7  | 80-100   | 2.0s     | 1.2x        |
| 0.7-1.0  | 100-127  | 3.0s     | 1.5x        |

## API Endpoints

### REST
- `GET /` - Service info
- `GET /health` - Health check
- `POST /compose` - Generate composition
- `GET /compositions` - List compositions
- `GET /compositions/{id}` - Get composition details

### WebSocket
- `WS /ws` - Real-time memory streaming
  - Client: `{"action": "start_stream"}`
  - Server: `{"type": "note_on", "note": "C4", "velocity": 80, ...}`

## Suno Integration (Phase 3)

When enabled, compositions can be sent to Suno AI for full song generation:

1. Extract lyrical themes from memory content
2. Generate style prompt from memory type distribution
3. Call Suno API with:
   ```json
   {
     "styles": "ambient electronic, cinematic, 120BPM",
     "lyrics": "[generated from memory themes]",
     "title": "Neural Symphony: Memory Cluster #42"
   }
   ```
4. Poll for completion
5. Download and serve audio

## Deployment

### Local Development
```bash
# Backend
cd backend
python -m venv venv
source venv/bin/activate
pip install -r requirements.txt
uvicorn main:app --reload --port 8765

# Frontend
cd frontend
npm install
npm run dev  # http://localhost:5173
```

### Production
- Backend: Gunicorn + Uvicorn workers
- Frontend: Vite build → nginx
- Cerebro: Already running via Hermes MCP

## Future Enhancements

1. **MIDI Export:** Download compositions as standard MIDI files
2. **Multi-track:** Separate tracks per memory type
3. **AI Lyrics:** LLM generates lyrics from memory content
4. **Live Mode:** Real-time memory access triggers notes
5. **Godot Export:** Standalone desktop app with better 3D
6. **Collaborative:** Multiple users, shared compositions
7. **Memory Clusters:** Auto-group related memories into movements

## Performance Considerations

- **Caching:** Composition results cached for 1 hour
- **WebSockets:** Use for real-time, REST for bulk
- **3D Limit:** Max 100 nodes in scene (LOD for performance)
- **Audio:** Web Audio API handles ~50 concurrent voices

## Security

- CORS enabled for localhost:5173
- No authentication (local-only deployment)
- Cerebro access via MCP (already authenticated)
- Suno API key in `.env` (never committed)

---

*Built on dual 4090 cloud compute, August 2026*