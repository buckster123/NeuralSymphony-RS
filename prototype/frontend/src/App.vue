<template>
  <div class="neural-symphony">
    <!-- 3D Visualization Canvas -->
    <div ref="canvasContainer" class="canvas-container"></div>
    
    <!-- Control Panel -->
    <div class="control-panel">
      <h1>Neural Symphony Studio</h1>
      <p class="subtitle">Transforming memories into music</p>
      
      <div class="controls">
        <button @click="generateComposition" :disabled="generating" class="btn-primary">
          {{ generating ? 'Composing...' : 'Generate Composition' }}
        </button>
        
        <div class="stats" v-if="composition">
          <div class="stat">
            <span class="label">Notes</span>
            <span class="value">{{ composition.note_count }}</span>
          </div>
          <div class="stat">
            <span class="label">Duration</span>
            <span class="value">{{ composition.duration.toFixed(1) }}s</span>
          </div>
          <div class="stat">
            <span class="label">Key</span>
            <span class="value">{{ composition.key }}</span>
          </div>
          <div class="stat">
            <span class="label">BPM</span>
            <span class="value">{{ composition.bpm }}</span>
          </div>
        </div>
      </div>
      
      <!-- Memory Type Legend -->
      <div class="legend">
        <h3>Memory Instruments</h3>
        <div class="legend-item" v-for="(instrument, type) in instrumentMap" :key="type">
          <span class="color-dot" :style="{ background: typeColors[type] }"></span>
          <span class="type-name">{{ type }}</span>
          <span class="instrument">{{ instrument }}</span>
        </div>
      </div>
    </div>
    
    <!-- Playing Status -->
    <div class="status-overlay" v-if="isPlaying">
      <div class="playing-indicator">
        <div class="pulse"></div>
        <span>Playing Symphony</span>
      </div>
    </div>
  </div>
</template>

<script>
import { ref, onMounted, onBeforeUnmount } from 'vue'
import * as THREE from 'three'

export default {
  name: 'App',
  setup() {
    const canvasContainer = ref(null)
    const generating = ref(false)
    const isPlaying = ref(false)
    const composition = ref(null)
    
    const instrumentMap = {
      episodic: 'piano',
      procedural: 'synth_lead',
      semantic: 'strings',
      affective: 'cello',
      prospective: 'flute',
      schematic: 'bass'
    }
    
    const typeColors = {
      episodic: '#FF6B6B',
      procedural: '#4ECDC4',
      semantic: '#45B7D1',
      affective: '#96CEB4',
      prospective: '#FFEAA7',
      schematic: '#DDA0DD'
    }
    
    let scene, camera, renderer, animationId
    let memoryNodes = []
    
    // Initialize 3D scene
    const initThreeJS = () => {
      if (!canvasContainer.value) return
      
      scene = new THREE.Scene()
      scene.background = new THREE.Color(0x0f0f1a)
      
      camera = new THREE.PerspectiveCamera(75, window.innerWidth / window.innerHeight, 0.1, 1000)
      camera.position.z = 30
      
      renderer = new THREE.WebGLRenderer({ antialias: true })
      renderer.setSize(window.innerWidth, window.innerHeight)
      canvasContainer.value.appendChild(renderer.domElement)
      
      // Add ambient light
      const ambientLight = new THREE.AmbientLight(0x404040, 2)
      scene.add(ambientLight)
      
      // Add point lights
      const light1 = new THREE.PointLight(0xFF6B6B, 2, 100)
      light1.position.set(10, 10, 10)
      scene.add(light1)
      
      const light2 = new THREE.PointLight(0x4ECDC4, 2, 100)
      light2.position.set(-10, -10, 10)
      scene.add(light2)
      
      // Create initial memory nodes
      createMemoryNodes(20)
      
      animate()
    }
    
    const createMemoryNodes = (count) => {
      memoryNodes = []
      
      for (let i = 0; i < count; i++) {
        const geometry = new THREE.SphereGeometry(0.8, 32, 32)
        const type = ['episodic', 'procedural', 'semantic', 'affective'][i % 4]
        const material = new THREE.MeshPhongMaterial({
          color: new THREE.Color(typeColors[type]),
          emissive: new THREE.Color(typeColors[type]),
          emissiveIntensity: 0.3,
          transparent: true,
          opacity: 0.8
        })
        
        const node = new THREE.Mesh(geometry, material)
        
        // Random position in sphere
        const theta = Math.random() * Math.PI * 2
        const phi = Math.acos(2 * Math.random() - 1)
        const radius = 10 + Math.random() * 15
        
        node.position.x = radius * Math.sin(phi) * Math.cos(theta)
        node.position.y = radius * Math.sin(phi) * Math.sin(theta)
        node.position.z = radius * Math.cos(phi)
        
        // Store metadata
        node.userData = {
          type,
          baseScale: 1,
          targetScale: 1,
          velocity: 0.01 + Math.random() * 0.02,
          phase: Math.random() * Math.PI * 2
        }
        
        scene.add(node)
        memoryNodes.push(node)
      }
    }
    
    const animate = () => {
      animationId = requestAnimationFrame(animate)
      
      // Rotate scene slowly
      scene.rotation.y += 0.001
      
      // Animate nodes
      const time = Date.now() * 0.001
      memoryNodes.forEach((node, i) => {
        // Pulsing effect
        const pulse = Math.sin(time * node.userData.velocity * 10 + node.userData.phase)
        const scale = node.userData.baseScale + pulse * 0.3
        
        node.scale.setScalar(scale)
        
        // Gentle orbit
        node.position.x *= 0.999
        node.position.y *= 0.999
        node.position.z *= 0.999
        
        // Highlight playing nodes
        if (isPlaying.value) {
          node.material.emissiveIntensity = 0.5 + pulse * 0.3
        } else {
          node.material.emissiveIntensity = 0.3
        }
      })
      
      renderer.render(scene, camera)
    }
    
    const generateComposition = async () => {
      generating.value = true
      
      try {
        const response = await fetch('/api/compose?limit=30&min_salience=0.3')
        const data = await response.json()
        
        if (data.composition_id) {
          composition.value = data
          
          // Update visualization with new composition
          if (memoryNodes.length > 0) {
            memoryNodes.forEach((node, i) => {
              if (i < data.note_count) {
                node.userData.targetScale = 1 + (data.memories?.[i]?.salience || 0.5) * 0.5
              }
            })
          }
          
          // Start playing
          isPlaying.value = true
          setTimeout(() => {
            isPlaying.value = false
          }, data.duration * 1000)
        }
      } catch (error) {
        console.error('Error generating composition:', error)
        alert('Failed to generate composition')
      } finally {
        generating.value = false
      }
    }
    
    // Handle window resize
    const handleResize = () => {
      if (!camera || !renderer) return
      
      camera.aspect = window.innerWidth / window.innerHeight
      camera.updateProjectionMatrix()
      renderer.setSize(window.innerWidth, window.innerHeight)
    }
    
    onMounted(() => {
      initThreeJS()
      window.addEventListener('resize', handleResize)
    })
    
    onBeforeUnmount(() => {
      if (animationId) {
        cancelAnimationFrame(animationId)
      }
      window.removeEventListener('resize', handleResize)
    })
    
    return {
      canvasContainer,
      generating,
      isPlaying,
      composition,
      instrumentMap,
      typeColors,
      generateComposition
    }
  }
}
</script>

<style scoped>
.neural-symphony {
  width: 100vw;
  height: 100vh;
  position: relative;
}

.canvas-container {
  width: 100%;
  height: 100%;
  position: absolute;
  top: 0;
  left: 0;
}

.control-panel {
  position: absolute;
  top: 20px;
  left: 20px;
  z-index: 10;
  background: rgba(15, 15, 26, 0.9);
  padding: 24px;
  border-radius: 12px;
  backdrop-filter: blur(10px);
  border: 1px solid rgba(255, 255, 255, 0.1);
  max-width: 320px;
}

h1 {
  font-size: 24px;
  font-weight: 700;
  margin-bottom: 8px;
  background: linear-gradient(135deg, #FF6B6B, #4ECDC4);
  -webkit-background-clip: text;
  -webkit-text-fill-color: transparent;
}

.subtitle {
  color: #888;
  font-size: 14px;
  margin-bottom: 24px;
}

.controls {
  margin-bottom: 24px;
}

.btn-primary {
  width: 100%;
  padding: 12px 24px;
  background: linear-gradient(135deg, #FF6B6B, #4ECDC4);
  border: none;
  border-radius: 8px;
  color: white;
  font-size: 16px;
  font-weight: 600;
  cursor: pointer;
  transition: transform 0.2s, box-shadow 0.2s;
}

.btn-primary:hover:not(:disabled) {
  transform: translateY(-2px);
  box-shadow: 0 4px 12px rgba(255, 107, 107, 0.4);
}

.btn-primary:disabled {
  opacity: 0.6;
  cursor: not-allowed;
}

.stats {
  display: grid;
  grid-template-columns: repeat(2, 1fr);
  gap: 12px;
  margin-top: 16px;
}

.stat {
  background: rgba(255, 255, 255, 0.05);
  padding: 12px;
  border-radius: 8px;
  text-align: center;
}

.stat .label {
  display: block;
  font-size: 12px;
  color: #888;
  margin-bottom: 4px;
}

.stat .value {
  display: block;
  font-size: 20px;
  font-weight: 700;
  color: #4ECDC4;
}

.legend {
  margin-top: 24px;
}

.legend h3 {
  font-size: 14px;
  margin-bottom: 12px;
  color: #aaa;
}

.legend-item {
  display: flex;
  align-items: center;
  gap: 8px;
  margin-bottom: 8px;
  font-size: 13px;
}

.color-dot {
  width: 12px;
  height: 12px;
  border-radius: 50%;
  flex-shrink: 0;
}

.type-name {
  flex: 1;
  color: #ccc;
}

.instrument {
  font-size: 12px;
  color: #666;
}

.status-overlay {
  position: absolute;
  bottom: 40px;
  left: 50%;
  transform: translateX(-50%);
  z-index: 10;
}

.playing-indicator {
  display: flex;
  align-items: center;
  gap: 12px;
  background: rgba(15, 15, 26, 0.9);
  padding: 16px 24px;
  border-radius: 30px;
  backdrop-filter: blur(10px);
  border: 1px solid rgba(78, 205, 196, 0.3);
}

.pulse {
  width: 12px;
  height: 12px;
  background: #4ECDC4;
  border-radius: 50%;
  animation: pulse 1.5s ease-in-out infinite;
}

@keyframes pulse {
  0%, 100% {
    transform: scale(1);
    opacity: 1;
  }
  50% {
    transform: scale(1.5);
    opacity: 0.5;
  }
}
</style>