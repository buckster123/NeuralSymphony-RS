# Neural Symphony Studio

**Interactive audio-visual experience that transforms memory patterns into music**

## Concept
Your CerebroCortex memories become a living symphony. Different memory types map to instruments, salience controls intensity, and associations create harmony. Real-time visualization shows memories "playing" as the music flows.

## Tech Stack
- **Backend:** FastAPI + Python
- **Frontend:** Vue 3 + Three.js
- **Audio Engine:** Suno AI + Web Audio API
- **Memory Layer:** CerebroCortex-RS
- **3D Visualization:** Three.js (web) / Godot (standalone)

## Architecture
```
CerebroCortex → Memory Extractor → Music Mapper → Suno/WebAudio → Visualization
     ↓                ↓                  ↓              ↓               ↓
  SQLite         FastAPI endpoint    Parameters      Generation      Three.js
```

## Phases
1. ✅ Project scaffold & core engine
2. Memory-to-music mapping system
3. Real-time 3D visualizer
4. Suno integration for full tracks
5. Export & polish

## Quick Start
```bash
# Backend
cd backend
python -m venv venv
source venv/bin/activate
pip install -r requirements.txt
uvicorn main:app --reload --host 0.0.0.0 --port 8765

# Frontend
cd frontend
npm install
npm run dev
```

## Features
- **Memory Themes:** Each memory type = unique instrument
- **Salience Dynamics:** Important memories = louder/more prominent
- **Association Harmony:** Linked memories create chord progressions
- **Real-time Viz:** 3D space where memories pulse with music
- **Suno Integration:** Generate full compositions from memory clusters
- **Export:** Save MIDI, audio, or full project files

## Why This Matters
This isn't just a demo - it's a new way to experience your digital memories. Instead of reading logs, you *hear* and *see* your cognitive patterns. It's meditation, art, and analytics in one.

---
*Built with ❤️ by Andre & Hermes on dual 4090 cloud compute*