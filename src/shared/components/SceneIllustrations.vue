<template>
  <div class="scene-illustration" :class="[`type--${type}`, `scene--${scene}`]">
    <!-- ==================== PDF UNLOCK ==================== -->
    <template v-if="scene === 'pdf-unlock'">
      <!-- Source: Locked PDF with heavy padlock & restriction marks -->
      <svg v-if="type === 'source'" viewBox="0 0 100 120" class="illus-svg" fill="none">
        <rect x="14" y="10" width="72" height="100" rx="6" fill="#f8fafc" stroke="#94a3b8" stroke-width="2" />
        <path d="M 64,10 L 86,32 L 64,32 Z" fill="#e2e8f0" stroke="#94a3b8" stroke-width="1.5" />
        <!-- Text lines -->
        <line x1="24" y1="42" x2="60" y2="42" stroke="#cbd5e1" stroke-width="3" stroke-linecap="round" />
        <line x1="24" y1="52" x2="52" y2="52" stroke="#cbd5e1" stroke-width="3" stroke-linecap="round" />
        <line x1="24" y1="62" x2="48" y2="62" stroke="#cbd5e1" stroke-width="3" stroke-linecap="round" />
        <!-- Locked Padlock in center -->
        <g class="padlock-group" transform="translate(34, 60)">
          <path d="M 8,14 V 8 C 8,3.6 11.6,0 16,0 C 20.4,0 24,3.6 24,8 V 14" stroke="#e11d48" stroke-width="3" stroke-linecap="round" />
          <rect x="2" y="14" width="28" height="22" rx="4" fill="#fb7185" stroke="#e11d48" stroke-width="2" />
          <circle cx="16" cy="24" r="2.5" fill="#881337" />
          <line x1="16" y1="26" x2="16" y2="30" stroke="#881337" stroke-width="2" stroke-linecap="round" />
        </g>
      </svg>
      <!-- Target: Unlocked PDF with radiant green shield & open lock -->
      <svg v-else viewBox="0 0 100 120" class="illus-svg" fill="none">
        <rect x="14" y="10" width="72" height="100" rx="6" fill="#f0fdf4" stroke="#4ade80" stroke-width="2" />
        <path d="M 64,10 L 86,32 L 64,32 Z" fill="#dcfce7" stroke="#4ade80" stroke-width="1.5" />
        <line x1="24" y1="42" x2="76" y2="42" stroke="#86efac" stroke-width="3" stroke-linecap="round" />
        <line x1="24" y1="52" x2="68" y2="52" stroke="#86efac" stroke-width="3" stroke-linecap="round" />
        <line x1="24" y1="62" x2="72" y2="62" stroke="#86efac" stroke-width="3" stroke-linecap="round" />
        <!-- Radiant Open Shield Badge -->
        <g class="shield-group" transform="translate(34, 60)">
          <path d="M 16,2 L 28,6 V 16 C 28,24 16,30 16,30 C 16,30 4,24 4,16 V 6 Z" fill="#22c55e" />
          <path d="M 10,16 L 14,20 L 22,12" stroke="#ffffff" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round" />
        </g>
        <circle cx="76" cy="74" r="2" fill="#22c55e" class="sparkle-dot" />
        <circle cx="22" cy="78" r="1.5" fill="#22c55e" class="sparkle-dot" />
      </svg>
    </template>

    <!-- ==================== PDF MERGE ==================== -->
    <template v-else-if="scene === 'pdf-merge'">
      <!-- Source: Stacking colored sheets -->
      <svg v-if="type === 'source'" viewBox="0 0 100 120" class="illus-svg" fill="none">
        <!-- Layer 1 -->
        <rect x="10" y="24" width="60" height="82" rx="4" fill="#ede9fe" stroke="#a78bfa" stroke-width="1.5" transform="rotate(-6 10 24)" />
        <!-- Layer 2 -->
        <rect x="24" y="16" width="60" height="82" rx="4" fill="#e0f2fe" stroke="#38bdf8" stroke-width="1.5" transform="rotate(4 24 16)" />
        <!-- Top Layer -->
        <rect x="18" y="20" width="64" height="84" rx="5" fill="#ffffff" stroke="#6366f1" stroke-width="2" />
        <line x1="26" y1="36" x2="68" y2="36" stroke="#c7d2fe" stroke-width="3" stroke-linecap="round" />
        <line x1="26" y1="46" x2="56" y2="46" stroke="#c7d2fe" stroke-width="3" stroke-linecap="round" />
        <line x1="26" y1="56" x2="72" y2="56" stroke="#c7d2fe" stroke-width="3" stroke-linecap="round" />
        <!-- Binder clip -->
        <rect x="42" y="14" width="16" height="10" rx="3" fill="#4f46e5" />
        <circle cx="50" cy="19" r="2" fill="#ffffff" />
      </svg>
      <!-- Target: Unified bound book/dossier with spine & ribbon -->
      <svg v-else viewBox="0 0 100 120" class="illus-svg" fill="none">
        <rect x="18" y="12" width="68" height="96" rx="5" fill="#f8fafc" stroke="#4f46e5" stroke-width="2" />
        <!-- Spine on left -->
        <rect x="18" y="12" width="10" height="96" rx="3" fill="#4f46e5" />
        <line x1="23" y1="20" x2="23" y2="100" stroke="#818cf8" stroke-width="1.5" stroke-dasharray="2 3" />
        <!-- Bookmark ribbon -->
        <path d="M 62,12 V 38 L 68,32 L 74,38 V 12 Z" fill="#f43f5e" />
        <!-- Content lines -->
        <line x1="36" y1="34" x2="56" y2="34" stroke="#4f46e5" stroke-width="3" stroke-linecap="round" />
        <line x1="36" y1="46" x2="76" y2="46" stroke="#cbd5e1" stroke-width="2.5" stroke-linecap="round" />
        <line x1="36" y1="56" x2="72" y2="56" stroke="#cbd5e1" stroke-width="2.5" stroke-linecap="round" />
        <line x1="36" y1="66" x2="68" y2="66" stroke="#cbd5e1" stroke-width="2.5" stroke-linecap="round" />
        <!-- Gold seal -->
        <circle cx="56" cy="86" r="10" fill="#fef08a" stroke="#eab308" stroke-width="1.5" />
        <text x="56" y="90" font-size="8" font-weight="bold" fill="#854d0e" text-anchor="middle">合订</text>
      </svg>
    </template>

    <!-- ==================== PDF COMPRESS ==================== -->
    <template v-else-if="scene === 'pdf-compress'">
      <!-- Source: Dense heavy document with weight tag -->
      <svg v-if="type === 'source'" viewBox="0 0 100 120" class="illus-svg" fill="none">
        <rect x="14" y="12" width="72" height="96" rx="6" fill="#f1f5f9" stroke="#64748b" stroke-width="2" />
        <!-- Heavy dense block lines -->
        <rect x="22" y="24" width="56" height="8" rx="2" fill="#94a3b8" />
        <rect x="22" y="36" width="56" height="8" rx="2" fill="#94a3b8" />
        <rect x="22" y="48" width="56" height="8" rx="2" fill="#94a3b8" />
        <rect x="22" y="60" width="56" height="8" rx="2" fill="#94a3b8" />
        <!-- Heavy weight badge -->
        <g transform="translate(25, 76)">
          <rect width="50" height="20" rx="4" fill="#334155" />
          <text x="25" y="14" font-size="9" font-weight="bold" fill="#ffffff" text-anchor="middle">24.8 MB</text>
        </g>
      </svg>
      <!-- Target: Feather-light streamlined document -->
      <svg v-else viewBox="0 0 100 120" class="illus-svg" fill="none">
        <rect x="14" y="12" width="72" height="96" rx="6" fill="#ecfdf5" stroke="#10b981" stroke-width="2" />
        <line x1="24" y1="28" x2="68" y2="28" stroke="#6ee7b7" stroke-width="2.5" stroke-linecap="round" />
        <line x1="24" y1="38" x2="60" y2="38" stroke="#6ee7b7" stroke-width="2.5" stroke-linecap="round" />
        <line x1="24" y1="48" x2="72" y2="48" stroke="#6ee7b7" stroke-width="2.5" stroke-linecap="round" />
        <!-- Lightning speed badge -->
        <g transform="translate(20, 68)">
          <rect width="60" height="24" rx="12" fill="#10b981" />
          <path d="M 12,12 L 18,5 L 17,11 L 22,11 L 14,19 L 15,13 Z" fill="#fef08a" />
          <text x="36" y="16" font-size="10" font-weight="bold" fill="#ffffff" text-anchor="middle">-72%</text>
        </g>
      </svg>
    </template>

    <!-- ==================== PDF ANTI-COPY ==================== -->
    <template v-else-if="scene === 'pdf-anticopy'">
      <!-- Source: Vulnerable copyable text -->
      <svg v-if="type === 'source'" viewBox="0 0 100 120" class="illus-svg" fill="none">
        <rect x="14" y="12" width="72" height="96" rx="6" fill="#f8fafc" stroke="#94a3b8" stroke-width="2" />
        <rect x="22" y="32" width="54" height="12" fill="#bfdbfe" opacity="0.6" />
        <line x1="24" y1="38" x2="72" y2="38" stroke="#3b82f6" stroke-width="2.5" stroke-linecap="round" />
        <line x1="24" y1="52" x2="64" y2="52" stroke="#94a3b8" stroke-width="2.5" stroke-linecap="round" />
        <line x1="24" y1="64" x2="56" y2="64" stroke="#94a3b8" stroke-width="2.5" stroke-linecap="round" />
        <!-- Mouse cursor copying -->
        <path d="M 68,40 L 78,50 L 73,51 L 76,57 L 73,58 L 70,52 L 66,54 Z" fill="#0f172a" stroke="#ffffff" stroke-width="1.2" />
      </svg>
      <!-- Target: Armored document with security grid & stamp -->
      <svg v-else viewBox="0 0 100 120" class="illus-svg" fill="none">
        <rect x="14" y="12" width="72" height="96" rx="6" fill="#f8fafc" stroke="#0284c7" stroke-width="2" />
        <!-- Watermark diagonal text -->
        <text x="50" y="55" font-size="10" font-weight="bold" fill="#0284c7" opacity="0.22" text-anchor="middle" transform="rotate(-30 50 55)">DOCSY SECURE</text>
        <line x1="24" y1="32" x2="72" y2="32" stroke="#bae6fd" stroke-width="2.5" stroke-linecap="round" />
        <line x1="24" y1="44" x2="64" y2="44" stroke="#bae6fd" stroke-width="2.5" stroke-linecap="round" />
        <line x1="24" y1="56" x2="68" y2="56" stroke="#bae6fd" stroke-width="2.5" stroke-linecap="round" />
        <!-- Official anti-tamper security badge -->
        <g transform="translate(30, 68)">
          <path d="M 20,2 L 34,6 V 16 C 34,26 20,32 20,32 C 20,32 6,26 6,16 V 6 Z" fill="#0284c7" />
          <circle cx="20" cy="15" r="4" fill="#ffffff" />
          <path d="M 20,13 V 17 M 18,15 H 22" stroke="#0284c7" stroke-width="1.5" stroke-linecap="round" />
        </g>
      </svg>
    </template>

    <!-- ==================== EVIDENCE ==================== -->
    <template v-else-if="scene === 'evidence'">
      <!-- Source: Case dossier file -->
      <svg v-if="type === 'source'" viewBox="0 0 100 120" class="illus-svg" fill="none">
        <path d="M 12,24 C 12,20 15,16 19,16 H 42 L 50,24 H 83 C 87,24 90,27 90,31 V 98 C 90,102 87,106 83,106 H 19 C 15,106 12,102 12,98 Z" fill="#fef3c7" stroke="#d97706" stroke-width="2" />
        <rect x="22" y="38" width="56" height="54" rx="4" fill="#ffffff" stroke="#fcd34d" stroke-width="1.5" />
        <rect x="28" y="46" width="30" height="6" rx="2" fill="#d97706" />
        <line x1="28" y1="60" x2="68" y2="60" stroke="#cbd5e1" stroke-width="2" stroke-linecap="round" />
        <line x1="28" y1="70" x2="62" y2="70" stroke="#cbd5e1" stroke-width="2" stroke-linecap="round" />
        <line x1="28" y1="80" x2="54" y2="80" stroke="#cbd5e1" stroke-width="2" stroke-linecap="round" />
      </svg>
      <!-- Target: Formal court catalog with dual-sided duplex pages & red evidence seal -->
      <svg v-else viewBox="0 0 100 120" class="illus-svg" fill="none">
        <!-- Facing pages -->
        <rect x="10" y="18" width="38" height="84" rx="3" fill="#ffffff" stroke="#e11d48" stroke-width="1.5" />
        <rect x="52" y="18" width="38" height="84" rx="3" fill="#ffffff" stroke="#e11d48" stroke-width="1.5" />
        <!-- Table lines -->
        <line x1="16" y1="30" x2="42" y2="30" stroke="#fecdd3" stroke-width="2" stroke-linecap="round" />
        <line x1="16" y1="40" x2="40" y2="40" stroke="#fecdd3" stroke-width="2" stroke-linecap="round" />
        <line x1="58" y1="30" x2="84" y2="30" stroke="#fecdd3" stroke-width="2" stroke-linecap="round" />
        <line x1="58" y1="40" x2="80" y2="40" stroke="#fecdd3" stroke-width="2" stroke-linecap="round" />
        <!-- Red round court evidence seal -->
        <circle cx="72" cy="74" r="13" stroke="#e11d48" stroke-width="2" fill="#fff1f2" />
        <circle cx="72" cy="74" r="10" stroke="#e11d48" stroke-width="1" stroke-dasharray="2 2" fill="none" />
        <text x="72" y="78" font-size="7" font-weight="bold" fill="#e11d48" text-anchor="middle">证据印</text>
        <!-- Page Number Pill -->
        <rect x="14" y="86" width="30" height="10" rx="3" fill="#f1f5f9" />
        <text x="29" y="94" font-size="6" fill="#475569" text-anchor="middle">P.001</text>
      </svg>
    </template>

    <!-- ==================== TEMPLATE ==================== -->
    <template v-else-if="scene === 'template'">
      <!-- Source: Word template with yellow variable placeholders -->
      <svg v-if="type === 'source'" viewBox="0 0 100 120" class="illus-svg" fill="none">
        <rect x="14" y="10" width="72" height="100" rx="6" fill="#f8fafc" stroke="#3b82f6" stroke-width="2" />
        <!-- Top Doc Ribbon -->
        <rect x="14" y="10" width="22" height="14" rx="2" fill="#2563eb" />
        <text x="25" y="20" font-size="8" font-weight="bold" fill="#ffffff" text-anchor="middle">W</text>
        <!-- Variable tags -->
        <rect x="24" y="36" width="46" height="12" rx="3" fill="#fef08a" stroke="#eab308" stroke-width="1" />
        <text x="47" y="45" font-size="7" fill="#713f12" text-anchor="middle">{客户名称}</text>
        <line x1="24" y1="58" x2="74" y2="58" stroke="#cbd5e1" stroke-width="2.5" stroke-linecap="round" />
        <rect x="24" y="68" width="40" height="12" rx="3" fill="#fef08a" stroke="#eab308" stroke-width="1" />
        <text x="44" y="77" font-size="7" fill="#713f12" text-anchor="middle">{签约日期}</text>
        <line x1="24" y1="90" x2="64" y2="90" stroke="#cbd5e1" stroke-width="2.5" stroke-linecap="round" />
      </svg>
      <!-- Target: Generated Word document with real filled values -->
      <svg v-else viewBox="0 0 100 120" class="illus-svg" fill="none">
        <rect x="14" y="10" width="72" height="100" rx="6" fill="#f8fafc" stroke="#2563eb" stroke-width="2" />
        <rect x="14" y="10" width="22" height="14" rx="2" fill="#2563eb" />
        <text x="25" y="20" font-size="8" font-weight="bold" fill="#ffffff" text-anchor="middle">W</text>
        <text x="24" y="44" font-size="8" font-weight="bold" fill="#1e293b">Docsy 科技</text>
        <line x1="24" y1="56" x2="74" y2="56" stroke="#94a3b8" stroke-width="2" stroke-linecap="round" />
        <line x1="24" y1="66" x2="70" y2="66" stroke="#94a3b8" stroke-width="2" stroke-linecap="round" />
        <text x="24" y="80" font-size="8" fill="#475569">2026-10-20</text>
        <!-- Red Authorized Stamp -->
        <circle cx="68" cy="84" r="12" stroke="#dc2626" stroke-width="1.8" fill="#fef2f2" opacity="0.9" />
        <text x="68" y="88" font-size="7" font-weight="bold" fill="#dc2626" text-anchor="middle">已盖章</text>
      </svg>
    </template>

    <!-- ==================== IMAGE PADDLER ==================== -->
    <template v-else-if="scene === 'image-paddler'">
      <!-- Source: Scattered polaroid photos -->
      <svg v-if="type === 'source'" viewBox="0 0 100 120" class="illus-svg" fill="none">
        <!-- Photo 1 -->
        <g transform="rotate(-10 35 45)">
          <rect x="15" y="20" width="44" height="52" rx="3" fill="#ffffff" stroke="#94a3b8" stroke-width="1.5" />
          <rect x="19" y="24" width="36" height="34" rx="2" fill="#bae6fd" />
          <circle cx="37" cy="41" r="6" fill="#0284c7" />
        </g>
        <!-- Photo 2 -->
        <g transform="rotate(8 55 60)">
          <rect x="36" y="38" width="46" height="54" rx="3" fill="#ffffff" stroke="#cbd5e1" stroke-width="1.5" />
          <rect x="40" y="42" width="38" height="36" rx="2" fill="#fbcfe8" />
          <path d="M 44,68 L 54,54 L 62,64 L 70,58 L 76,68 Z" fill="#db2777" />
        </g>
      </svg>
      <!-- Target: Padded & aligned layout sheet -->
      <svg v-else viewBox="0 0 100 120" class="illus-svg" fill="none">
        <rect x="14" y="10" width="72" height="100" rx="6" fill="#ffffff" stroke="#0ea5e9" stroke-width="2" />
        <!-- Photo grid with clear margins -->
        <rect x="22" y="20" width="26" height="34" rx="2" fill="#f0f9ff" stroke="#38bdf8" stroke-width="1.2" />
        <rect x="52" y="20" width="26" height="34" rx="2" fill="#f0f9ff" stroke="#38bdf8" stroke-width="1.2" />
        <rect x="22" y="60" width="26" height="34" rx="2" fill="#f0f9ff" stroke="#38bdf8" stroke-width="1.2" />
        <rect x="52" y="60" width="26" height="34" rx="2" fill="#f0f9ff" stroke="#38bdf8" stroke-width="1.2" />
        <!-- Alignment Guide Crosshairs -->
        <circle cx="50" cy="57" r="3" stroke="#0ea5e9" stroke-width="1" stroke-dasharray="1 1" fill="none" />
      </svg>
    </template>

    <!-- ==================== VIDEO EXTRACT ==================== -->
    <template v-else-if="scene === 'video-extract'">
      <!-- Source: Film strip -->
      <svg v-if="type === 'source'" viewBox="0 0 100 120" class="illus-svg" fill="none">
        <rect x="20" y="10" width="60" height="100" rx="4" fill="#0f172a" />
        <!-- Sprocket holes -->
        <rect x="24" y="18" width="6" height="8" rx="1.5" fill="#ffffff" />
        <rect x="24" y="34" width="6" height="8" rx="1.5" fill="#ffffff" />
        <rect x="24" y="50" width="6" height="8" rx="1.5" fill="#ffffff" />
        <rect x="24" y="66" width="6" height="8" rx="1.5" fill="#ffffff" />
        <rect x="24" y="82" width="6" height="8" rx="1.5" fill="#ffffff" />
        <rect x="24" y="98" width="6" height="8" rx="1.5" fill="#ffffff" />

        <rect x="70" y="18" width="6" height="8" rx="1.5" fill="#ffffff" />
        <rect x="70" y="34" width="6" height="8" rx="1.5" fill="#ffffff" />
        <rect x="70" y="50" width="6" height="8" rx="1.5" fill="#ffffff" />
        <rect x="70" y="66" width="6" height="8" rx="1.5" fill="#ffffff" />
        <rect x="70" y="82" width="6" height="8" rx="1.5" fill="#ffffff" />
        <rect x="70" y="98" width="6" height="8" rx="1.5" fill="#ffffff" />

        <!-- Film Frames -->
        <rect x="34" y="24" width="32" height="26" rx="2" fill="#1e293b" />
        <polygon points="46,32 56,37 46,42" fill="#38bdf8" />
        <rect x="34" y="60" width="32" height="26" rx="2" fill="#1e293b" />
        <polygon points="46,68 56,73 46,78" fill="#38bdf8" />
      </svg>
      <!-- Target: Keyframe snapshot cards with timestamps -->
      <svg v-else viewBox="0 0 100 120" class="illus-svg" fill="none">
        <rect x="14" y="14" width="72" height="42" rx="4" fill="#ffffff" stroke="#0284c7" stroke-width="1.8" />
        <rect x="18" y="18" width="64" height="26" rx="2" fill="#e0f2fe" />
        <circle cx="50" cy="31" r="5" fill="#0284c7" />
        <rect x="20" y="47" width="36" height="7" rx="2" fill="#0284c7" />
        <text x="38" y="53" font-size="5.5" font-weight="bold" fill="#ffffff" text-anchor="middle">00:01:24</text>

        <rect x="14" y="64" width="72" height="42" rx="4" fill="#ffffff" stroke="#0284c7" stroke-width="1.8" />
        <rect x="18" y="68" width="64" height="26" rx="2" fill="#e0f2fe" />
        <circle cx="50" cy="81" r="5" fill="#0284c7" />
        <rect x="20" y="97" width="36" height="7" rx="2" fill="#0284c7" />
        <text x="38" y="103" font-size="5.5" font-weight="bold" fill="#ffffff" text-anchor="middle">00:02:10</text>
      </svg>
    </template>

    <!-- ==================== GENERIC / FALLBACK ==================== -->
    <template v-else>
      <svg viewBox="0 0 100 120" class="illus-svg" fill="none">
        <rect x="14" y="10" width="72" height="100" rx="6" fill="#f8fafc" stroke="#94a3b8" stroke-width="2" />
        <line x1="24" y1="32" x2="68" y2="32" stroke="#cbd5e1" stroke-width="3" stroke-linecap="round" />
        <line x1="24" y1="46" x2="76" y2="46" stroke="#cbd5e1" stroke-width="2.5" stroke-linecap="round" />
        <line x1="24" y1="58" x2="60" y2="58" stroke="#cbd5e1" stroke-width="2.5" stroke-linecap="round" />
        <line x1="24" y1="70" x2="70" y2="70" stroke="#cbd5e1" stroke-width="2.5" stroke-linecap="round" />
      </svg>
    </template>
  </div>
</template>

<script setup>
defineProps({
  scene: { type: String, default: 'generic' },
  type: { type: String, default: 'source', validator: (v) => ['source', 'target'].includes(v) },
})
</script>

<style scoped>
.scene-illustration {
  width: 100%;
  height: 100%;
  display: flex;
  align-items: center;
  justify-content: center;
}

.illus-svg {
  width: 100%;
  height: 100%;
  max-height: 108px;
  overflow: visible;
  filter: drop-shadow(0 2px 6px rgba(0, 0, 0, 0.05));
  transition: transform 0.3s ease;
}

.scene-illustration:hover .illus-svg {
  transform: scale(1.03);
}

.sparkle-dot {
  animation: pulse-sparkle 1.8s ease-in-out infinite alternate;
}

@keyframes pulse-sparkle {
  0% { transform: scale(0.8); opacity: 0.5; }
  100% { transform: scale(1.4); opacity: 1; }
}
</style>
