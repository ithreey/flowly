# UI Redesign Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Apply the confirmed Flowly dark developer-workbench visual system to the existing Vue + Element Plus application.

**Architecture:** Keep the existing Vue pages, router, stores, and Tauri commands. Rework the global shell in `App.vue`, then restyle the high-value screens in place with scoped CSS and small template changes.

**Tech Stack:** Vue 3, Element Plus, Pinia, Tauri v2, Vite.

---

### Task 1: Global Shell

**Files:**

- Modify: `app/src/App.vue`

- [x] Replace the plain white sidebar with a dark workbench sidebar.
- [x] Add a top status bar with route title, description, proxy state, and start/stop action.
- [x] Add global dark Element Plus theme overrides.

### Task 2: Traffic Monitor

**Files:**

- Modify: `app/src/pages/Monitor.vue`

- [x] Convert the toolbar to the confirmed compact workbench layout.
- [x] Restyle method/status cells as color-coded tags.
- [x] Split URL display into host and path for scanability.
- [x] Restyle context menu and empty states.

### Task 3: Detail Drawer

**Files:**

- Modify: `app/src/components/DetailDrawer.vue`

- [x] Add summary metrics for status, duration, request size, and response size.
- [x] Restyle headers/body/code blocks for dark reading.
- [x] Add a timeline tab with currently available timing data.

### Task 4: Secondary Pages

**Files:**

- Modify: `app/src/pages/Rules.vue`
- Modify: `app/src/pages/Settings.vue`
- Modify: `app/src/pages/Certificates.vue`
- Modify: `app/src/pages/AppSettings.vue`

- [x] Convert pages from default card/table styling into dark workbench panels.
- [x] Add rule summaries and status cards where data is already available.
- [x] Keep existing business logic unchanged.

### Task 5: Verification

**Files:**

- Check: `app/src/**/*.vue`

- [x] Run `npm run build` in `app`.
- [x] Fix any compile errors.
- [x] Review the changed files against the design system.
