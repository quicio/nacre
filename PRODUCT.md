# Product

<!-- impeccable:product-schema 1 -->

## Platform

web

## Stack

Astro (salida estática), contenido en Markdown, desplegado en GitHub Pages con GitHub Actions. El idioma por defecto es español; el inglés se ofrece como versión conmutable.

## Users

Tres audiencias, en este orden:

1. **Músicos experimentales y productores de electrónica** que buscan un instrumento generativo con identidad sonora propia, no otro sintetizador.
2. **Desarrolladores de audio y Rust** que evalúan el proyecto por su arquitectura, su disciplina de tiempo real y su código.
3. **Curadores, sellos pequeños y comunidad de arte generativo** que necesitan entender en un minuto qué hace distinto a Nacre.

La escena real: alguien frente a un escritorio oscuro, con auriculares, buscando una textura o un patrón que no sabe cómo pedir; y alguien que abre el repositorio para juzgar si el motor está bien construido.

## Product Purpose

Nacre es un instrumento musical generativo escrito en Rust: un sistema que produce, transforma y hace evolucionar sonido, en lugar de reproducir una canción programada. Existe para explorar composición generativa, ritmo procedural y diseño sonoro modular con una calidad sonora que justifique el instrumento.

El éxito del MVP: tres renders de referencia que alguien quiera escuchar completos, una pieza de diez minutos que conserve identidad, y un agente que traduzca una intención musical en un cambio audible y reversible.

## Positioning

No es un DAW, ni un secuenciador, ni un modular virtual. Su mecanismo propio es un motor generativo de primera clase —eventos, semillas, evolución con presupuesto de cambio— acoplado a un motor de audio en tiempo real y a una API de agente transaccional. El instrumento no pide notas: pide comportamiento.

## Operating Context

- El producto final es una app de escritorio nativa (standalone primero, plugin VST3/CLAP después).
- El sitio público es la vitrina del proyecto: estado real, roadmap, arquitectura, decisiones, artefactos.
- El proyecto está en fase de planificación (M0): no hay binario ni capturas reales.
- El trabajo se documenta con OpenSpec; el roadmap y los ADRs son públicos.
- La licencia del producto aún no está decidida y no debe afirmarse en la web.

## Capabilities and Constraints

**Existe hoy:** visión, principios, arquitectura, 8 ADRs, roadmap por milestones y slices, cuatro changes de OpenSpec con 27 requisitos y 56 escenarios; el repositorio y su CI están especificados, no implementados.

**En construcción (roadmap):** M1 First Sound (audio, voz, parámetros, efectos, patch), M2 Generative Runtime, M3 Agent Integration, M4 plugin/MIDI/performance.

**Restricciones técnicas que la web puede afirmar:** Rust, tiempo real seguro (sin asignación ni locks en el callback), determinismo por semilla, Clean Architecture, agente independiente del proveedor de IA.

**No afirmar:** que la app existe, fechas de entrega, precios, licencia, pruebas de usuario, métricas de rendimiento no medidas.

## Brand Commitments

- Nombre: **NACRE** (nombre de trabajo; puede cambiar antes de publicar).
- Metáfora central: el nácar — capas sucesivas de crecimiento, iridiscencia, estructura oculta.
- Dirección creativa vinculante: **Organic Futurism / Synthetic Naturalism**. Referencias: estructuras biológicas microscópicas, minerales iridiscentes, formaciones geológicas, escultura biomórfica, instrumentos científicos experimentales, arte generativo, diseño industrial minimalista, arquitectura especulativa.
- Personalidad: inteligente, misteriosa, sofisticada, experimental, orgánica, precisa, silenciosamente futurista.
- Prohibido: formas de onda como logo principal, barras de ecualizador, notas musicales, ondas de sonido genéricas, neón cyberpunk, gradientes morado-azul de SaaS, HUD futurista, exceso de glow, estética "IA genérica", look juguetón o comercial.
- Tema oscuro primero; el claro debe funcionar.
- El logo debe seguir siendo reconocible sin color ni degradados.

## Evidence on Hand

- Documentación real en `docs/`: visión, principios, arquitectura, ADRs, riesgos, evaluación de dependencias.
- Especificaciones reales en `openspec/changes/`: 27 requisitos, 56 escenarios.
- Estado verificable: 0 % implementado; M0 en curso.
- **No existen capturas de la app.** Cualquier imagen de interfaz en la web es un mockup conceptual y debe estar etiquetada como tal.
- No hay audio publicado todavía; los renders de referencia llegarán con el gate de M1.

## Product Principles

1. El sonido va primero: ninguna feature compensa un motor que suena genérico.
2. Generativo por diseño: la variación tiene escala temporal, semilla y presupuesto.
3. Modular pero coherente: relaciones musicales, no un lienzo infinito de cables.
4. Agent-native, no agent-dependent: el instrumento funciona completo sin IA.
5. La incertidumbre se publica: hipótesis, spikes y gates visibles, no certezas inventadas.

## Accessibility & Inclusion

- Objetivo WCAG 2.2 AA en la web: contraste, foco visible, navegación por teclado, texto alternativo.
- Respetar `prefers-reduced-motion`: ninguna animación esencial.
- La web debe leerse bien sin JavaScript; la conmutación de idioma no puede bloquear el contenido.
- Español e inglés con paridad de contenido.
