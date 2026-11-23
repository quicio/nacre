export type Lang = 'es' | 'en';

export const LANGS: Lang[] = ['es', 'en'];

export const GITHUB = 'https://github.com/hugomoraga/nacre';

export type Milestone = {
  id: string;
  name: string;
  subtitle: string;
  state: 'active' | 'planned';
  gate: string;
  slices: { code: string; title: string }[];
};

export const MILESTONES: Milestone[] = [
  {
    id: 'M0',
    name: 'Fundación',
    subtitle: 'Foundation',
    state: 'active',
    gate: 'Revisión de arquitectura y specs',
    slices: [
      { code: '001', title: 'Workspace, límites y CI' },
    ],
  },
  {
    id: 'M1',
    name: 'Primer sonido',
    subtitle: 'First Sound',
    state: 'planned',
    gate: 'Tres renders de referencia y 30 min sin glitches',
    slices: [
      { code: '002', title: 'Fundación de audio' },
      { code: '003', title: 'Síntesis de voz' },
      { code: '004', title: 'Sistema de parámetros' },
      { code: '005', title: 'Espacio y carácter' },
      { code: '006', title: 'Documento de patch' },
    ],
  },
  {
    id: 'M2',
    name: 'Runtime generativo',
    subtitle: 'Generative Runtime',
    state: 'planned',
    gate: 'Pieza de 10 minutos con identidad',
    slices: [
      { code: '007', title: 'Reloj y scheduler' },
      { code: '008', title: 'Generadores I' },
      { code: '009', title: 'Generadores II' },
      { code: '010', title: 'Evolución' },
      { code: '011', title: 'Composición y escenas' },
    ],
  },
  {
    id: 'M3',
    name: 'Agente',
    subtitle: 'Agent',
    state: 'planned',
    gate: 'Intención musical a patch reversible',
    slices: [
      { code: '012', title: 'API de agente' },
      { code: '013', title: 'Transacciones' },
      { code: '014', title: 'Deshacer y rehacer' },
      { code: '015', title: 'Adaptador MCP' },
    ],
  },
  {
    id: 'M4',
    name: 'Instrumento',
    subtitle: 'Instrument',
    state: 'planned',
    gate: 'Plugin validado en hosts objetivo',
    slices: [
      { code: '016', title: 'Plugin VST3 / CLAP' },
      { code: '017', title: 'MIDI' },
      { code: '018', title: 'Performance' },
      { code: '019', title: 'Presets' },
    ],
  },
];

export const FACTS = [
  { value: '27', code: 'REQ' },
  { value: '56', code: 'ESC' },
  { value: '8', code: 'ADR' },
];

export const FACT_LABELS: Record<Lang, string[]> = {
  es: ['Requisitos', 'Escenarios', 'Decisiones'],
  en: ['Requirements', 'Scenarios', 'Decisions'],
};

export const COPY = {
  es: {
    htmlLang: 'es',
    meta: {
      title: 'NACRE — instrumento musical generativo',
      description:
        'Un instrumento musical generativo escrito en Rust. Sistemas que producen música, no canciones programadas. En desarrollo: M0, fundación.',
      ogAlt: 'Capas de nácar iridiscente sobre negro',
    },
    nav: {
      roadmap: 'Roadmap',
      architecture: 'Arquitectura',
      specs: 'OpenSpec',
      github: 'GitHub',
      switch: 'EN',
      switchLabel: 'Read in English',
      skip: 'Saltar al contenido',
    },
    hero: {
      kicker: 'Instrumento musical generativo',
      titleA: 'Un sistema',
      titleB: 'que genera',
      titleC: 'música',
      lede: 'No programa canciones. Construye sistemas capaces de producir música: capas, semillas y evolución con identidad.',
      cta: 'Explorar el roadmap',
      ctaSecondary: 'Leer el código',
      statusLabel: 'Estado del proyecto',
      statusValue: 'M0 — Fundación',
      factsTitle: 'Documentado, no prometido',
      plateAlt:
        'Láminas de nácar iridiscente superpuestas sobre fondo negro, con anotaciones técnicas',
      annotationOne: '48 kHz · 256 frames',
      annotationTwo: 'callback · 0 alloc',
      annotationThree: 'seed → hash',
      probe: 'Sonda',
      depth: 'Profundidad',
    },
    timeline: {
      label: 'Recorrido',
      active: 'en curso',
      planned: 'planificado',
    },
    roadmap: {
      kicker: 'Roadmap',
      title: 'Construir de oído, no de fe',
      lede: 'Cada tramo termina en un gate verificable. Si el sonido no convence, se corrige el motor antes de añadir funciones. El primer tramo está especificado en OpenSpec; el resto es secuencia, no promesa.',
      sliceLabel: 'Slices',
      gateLabel: 'Gate',
      stateActive: 'En curso',
      statePlanned: 'No iniciado',
      note: 'Estado real a 22 de noviembre de 2026. Cero código de audio escrito: la primera tarea es el workspace.',
    },
    features: {
      kicker: 'Principios en práctica',
      title: 'Lo que el instrumento promete',
      lede: 'Seis decisiones que separan un sistema generativo de un sintetizador con un botón aleatorio.',
      items: [
        {
          title: 'Tiempo real, sin excusas',
          body: 'El callback de audio no asigna memoria, no toma locks, no hace I/O y no puede entrar en pánico. Todo lo demás ocurre fuera.',
        },
        {
          title: 'Determinismo por semilla',
          body: 'Misma semilla, mismo estado, mismo audio. El render offline se compara por hash, no de oído.',
        },
        {
          title: 'Generativo de primera clase',
          body: 'El motor generativo produce eventos musicales, no muestras. Tiene estado, relaciones y presupuesto de cambio para no perder la identidad.',
        },
        {
          title: 'Dominio independiente',
          body: 'El dominio no conoce audio, interfaz ni modelos de lenguaje. La infraestructura implementa puertos; nunca al revés.',
        },
        {
          title: 'Agente transaccional',
          body: 'Inspecciona, propone, valida, audiciona y revierte. Nunca muta el estado vivo por su cuenta.',
        },
        {
          title: 'Un motor, dos formatos',
          body: 'Standalone primero; plugin después. El mismo runtime, sin una segunda arquitectura escondida.',
        },
      ],
    },
    architecture: {
      kicker: 'Arquitectura',
      title: 'Capas que apuntan hacia dentro',
      lede: 'Clean Architecture, con una excepción explícita: el audio en tiempo real no es un caso de uso. Se separa el plano de control del plano de datos y esa frontera se diseña como un contrato propio.',
      layers: [
        { name: 'Presentación', detail: 'App standalone, plugin. Comandos y view models. Cero DSP.' },
        { name: 'Aplicación', detail: 'Casos de uso, puertos, transacciones, validación.' },
        { name: 'Dominio', detail: 'Patch, composición, performance, parámetros. Sin dependencias externas.' },
      ],
      side: { name: 'Infraestructura', detail: 'cpal, ficheros, MIDI, MCP. Implementa puertos definidos hacia dentro.' },
      rule: 'El dominio no importa nada de fuera. Si una capa exterior hace falta, se invierte la dependencia.',
      link: 'Ver el grafo de dependencias',
    },
    interface: {
      kicker: 'Interfaz',
      title: 'El instrumento, todavía en plano',
      lede: 'No hay binario ni capturas reales. Estas son maquetas conceptuales de la dirección visual: sirven para discutir jerarquía y densidad, no para afirmar que la app existe.',
      concept: 'Concepto',
      captionOne: 'Vista de instrumento: transporte, macros y lectura de estado.',
      captionTwo: 'Editor de patch: nodos tipados, modulación y parámetros.',
    },
    specs: {
      kicker: 'Especificaciones',
      title: 'Contratos antes que código',
      lede: 'Cada comportamiento se escribe como un requisito de OpenSpec con escenarios verificables. Si la implementación cambia sin cambiar lo observable, la spec no se toca.',
      reqLabel: 'Ejemplo real',
      reqTitle: 'Contrato de tiempo real del callback',
      reqBody:
        'El callback de audio debe completar su trabajo sin asignar memoria, sin adquirir locks, sin realizar I/O, sin compilar el grafo y sin entrar en pánico, con un coste acotado por bloque.',
      contentsLabel: 'Qué contiene una spec',
      contents: [
        'Objetivo y alcance',
        'No-alcance explícito',
        'Requisitos normativos',
        'Escenarios verificables',
        'Criterios de aceptación',
      ],
      scenarioOneWhen: 'el callback procesa 10 000 bloques con parámetros que cambian',
      scenarioOneThen: 'el contador de asignaciones permanece en cero',
      scenarioTwoWhen: 'un parámetro llega como NaN o infinito',
      scenarioTwoThen: 'se sustituye por un valor seguro y la salida sigue siendo finita',
      link: 'Leer los cambios en OpenSpec',
    },
    close: {
      title: 'El instrumento no existe todavía. La arquitectura sí.',
      body: 'Si te interesa el audio en tiempo real, la composición generativa o los agentes que no rompen el estado, el repositorio está abierto y el roadmap se puede auditar.',
      cta: 'Seguir el desarrollo en GitHub',
      secondary: 'Leer la visión',
    },
    footer: {
      note: 'NACRE es un nombre de trabajo. Licencia por definir.',
      built: 'Documentado con OpenSpec · Escrito en Rust',
      status: 'M0 en curso',
    },
  },
  en: {
    htmlLang: 'en',
    meta: {
      title: 'NACRE — generative musical instrument',
      description:
        'A generative musical instrument written in Rust. Systems that produce music, not programmed songs. In development: M0, foundation.',
      ogAlt: 'Iridescent mother-of-pearl layers on black',
    },
    nav: {
      roadmap: 'Roadmap',
      architecture: 'Architecture',
      specs: 'OpenSpec',
      github: 'GitHub',
      switch: 'ES',
      switchLabel: 'Leer en español',
      skip: 'Skip to content',
    },
    hero: {
      kicker: 'Generative musical instrument',
      titleA: 'A system',
      titleB: 'that generates',
      titleC: 'music',
      lede: 'It does not program songs. It builds systems capable of producing music: layers, seeds and evolution with identity.',
      cta: 'Explore the roadmap',
      ctaSecondary: 'Read the code',
      statusLabel: 'Project status',
      statusValue: 'M0 — Foundation',
      factsTitle: 'Documented, not promised',
      plateAlt:
        'Overlapping iridescent mother-of-pearl sheets on black, with technical annotations',
      annotationOne: '48 kHz · 256 frames',
      annotationTwo: 'callback · 0 alloc',
      annotationThree: 'seed → hash',
      probe: 'Probe',
      depth: 'Depth',
    },
    timeline: {
      label: 'Trajectory',
      active: 'in progress',
      planned: 'planned',
    },
    roadmap: {
      kicker: 'Roadmap',
      title: 'Build by ear, not by faith',
      lede: 'Every stage ends at a verifiable gate. If the sound does not convince, the engine is fixed before features are added. The first stage is specified in OpenSpec; the rest is sequence, not promise.',
      sliceLabel: 'Slices',
      gateLabel: 'Gate',
      stateActive: 'In progress',
      statePlanned: 'Not started',
      note: 'Real status as of 22 November 2026. Zero audio code written: the first task is the workspace.',
    },
    features: {
      kicker: 'Principles in practice',
      title: 'What the instrument promises',
      lede: 'Six decisions that separate a generative system from a synthesizer with a randomize button.',
      items: [
        {
          title: 'Real time, no excuses',
          body: 'The audio callback does not allocate, take locks, perform I/O or panic. Everything else happens outside it.',
        },
        {
          title: 'Determinism by seed',
          body: 'Same seed, same state, same audio. Offline renders are compared by hash, not by ear.',
        },
        {
          title: 'Generative first',
          body: 'The generative engine emits musical events, not samples. It has state, relationships and a change budget so identity survives.',
        },
        {
          title: 'Independent domain',
          body: 'The domain knows nothing about audio, UI or language models. Infrastructure implements ports, never the reverse.',
        },
        {
          title: 'Transactional agent',
          body: 'It inspects, proposes, validates, auditions and reverts. It never mutates live state on its own.',
        },
        {
          title: 'One engine, two formats',
          body: 'Standalone first; plugin later. The same runtime, without a second hidden architecture.',
        },
      ],
    },
    architecture: {
      kicker: 'Architecture',
      title: 'Layers that point inward',
      lede: 'Clean Architecture, with one explicit exception: real-time audio is not a use case. The control plane is separated from the data plane, and that boundary is designed as its own contract.',
      layers: [
        { name: 'Presentation', detail: 'Standalone app, plugin. Commands and view models. Zero DSP.' },
        { name: 'Application', detail: 'Use cases, ports, transactions, validation.' },
        { name: 'Domain', detail: 'Patch, composition, performance, parameters. No external dependencies.' },
      ],
      side: { name: 'Infrastructure', detail: 'cpal, files, MIDI, MCP. Implements ports defined inward.' },
      rule: 'The domain imports nothing from outside. When an outer layer is needed, the dependency is inverted.',
      link: 'See the dependency graph',
    },
    interface: {
      kicker: 'Interface',
      title: 'The instrument, still on the drawing board',
      lede: 'There is no binary and no real capture yet. These are concept mockups of the visual direction: they exist to discuss hierarchy and density, not to claim the app ships.',
      concept: 'Concept',
      captionOne: 'Instrument view: transport, macros and state readout.',
      captionTwo: 'Patch editor: typed nodes, modulation and parameters.',
    },
    specs: {
      kicker: 'Specifications',
      title: 'Contracts before code',
      lede: 'Every behaviour is written as an OpenSpec requirement with verifiable scenarios. If the implementation changes without changing what is observable, the spec does not move.',
      reqLabel: 'A real example',
      reqTitle: 'Real-time callback contract',
      reqBody:
        'The audio callback must complete its work without allocating memory, acquiring locks, performing I/O, compiling the graph or panicking, with bounded cost per block.',
      contentsLabel: 'What a spec contains',
      contents: [
        'Objective and scope',
        'Explicit non-goals',
        'Normative requirements',
        'Verifiable scenarios',
        'Acceptance criteria',
      ],
      scenarioOneWhen: 'the callback processes 10,000 blocks with changing parameters',
      scenarioOneThen: 'the allocation counter stays at zero',
      scenarioTwoWhen: 'a parameter arrives as NaN or infinity',
      scenarioTwoThen: 'it is replaced by a safe value and the output stays finite',
      link: 'Read the OpenSpec changes',
    },
    close: {
      title: 'The instrument does not exist yet. The architecture does.',
      body: 'If real-time audio, generative composition or agents that do not corrupt state interest you, the repository is open and the roadmap can be audited.',
      cta: 'Follow development on GitHub',
      secondary: 'Read the vision',
    },
    footer: {
      note: 'NACRE is a working name. Licence to be decided.',
      built: 'Documented with OpenSpec · Written in Rust',
      status: 'M0 in progress',
    },
  },
} as const;

export type Copy = (typeof COPY)['es'];
