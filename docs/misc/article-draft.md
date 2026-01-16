# AFK: A Laid-Back Way to Work with AI on Bigger Features

The speed at which AI coding tools are improving is impossible to ignore. Even [Linus Torvalds is having a go](https://www.theregister.com/2026/01/13/linus_torvalds_vibe_coding/)! If the creator of Linux is experimenting with AI-assisted coding, if you're not already, it's time to pay attention.

I've been in the software industry for quite a while, and AI tooling has been a genuine game-changer for my productivity over the last couple of years. It's helped me branch out into different languages, ship side projects I'd been meaning to build for years, and explore ideas I wouldn't have tackled alone. My [synth-tools](https://github.com/m0nkmaster/synth-tools) project - a browser-based synthesiser - is a good example. Without AI assistance, that would still be on my "one day" list.

But when you start using these tools on larger features, an anti-pattern can emerge.

## The Problem: Context Bloat

Long AI chat sessions get messy.

You're building something substantial - a new web app, a major feature, a multi-step refactor. You start prompting. The AI responds. You clarify. It revises, it's going well. You nudge the direction. It adjusts. Five or six exchanges in, the conversation is sprawling. Even with 200,000-token context windows, you can *feel* it struggling.

The model starts forgetting what you agreed earlier. It contradicts itself. It revisits decisions you already made. The output quality degrades, and you spend more time steering than thinking.

This is **context exhaustion**. The AI has filled its working memory with false starts, abandoned approaches, and conversational noise. It's become bloated and confused.

## Discovering the Ralph Approach

That's when I came across something called the **Ralph Wiggum approach** - or just "Ralph" for short.

The concept was first articulated by [Geoffrey Huntley](https://ghuntley.com/ralph/) in late 2025, and it's been gaining traction ever since. Matt Pocock covered it on [AI Hero](https://aihero.dev/), Ibrahim Pima did a deep dive on DEV Community, and it's showing up in more and more developer conversations.

The idea is named after Ralph Wiggum from *The Simpsons* - a character who approaches each moment with fresh-eyed obliviousness, unburdened by what came before. In AI terms, that forgetfulness is the *feature*, not the bug.

Here's the core of it:

1. **Fresh context every iteration** - Start each task with a brand-new AI instance
2. **One task at a time** - Kanban-style: pick up a ticket, complete it, move on
3. **Memory through files, not conversation** - Progress persists via git commits and  concisely captured learnings, not chat history.

No more dragging old context forward. No more rescuing the model mid-flight. Each task gets exactly the context it needs, and nothing more.

## Two Pillars: Context and Focus

Having been in the industry long enough to see patterns repeat, I immediately saw parallels with good engineering practice.

### 1. Context Without Bloat

In the Ralph approach, context is something you *prepare*, not something you accumulate. You don't patch it mid-run. You don't layer requirement on top of requirement until the developer loses the plot - or walks out!

Each iteration starts clean. If the output is off, you adjust the inputs explicitly and run again. That makes failures *useful* - they tell you something was unclear in the context, not that the AI had a bad day after a long conversation.

### 2. Focus on One Thing at a Time

The other key is focus. Each task is clearly defined and narrow. Not "build the dashboard" but "design the data model". Not "add auth" but "define the auth flow and constraints".

This dramatically reduces hallucinations and off-topic behaviour. The model has less room to wander because the scope is tight.

If you've ever used Kanban properly, this will feel familiar. One card. One goal. Finish it. Move on. It's not an AI idea - it's a good engineering idea, applied to AI.

## Why I Built afk

I read the articles. I tried the pattern manually with shell scripts. It worked well. But it was fiddly. I wanted something that made the Ralph approach *enjoyable* - a tool that encouraged people to give it a go.

That's why I built **afk**.

It's a small, tool-agnostic CLI that sits outside any specific AI app. It doesn't care if you're using Claude, Cursor, Codex, or something else... It just runs the pattern.

The workflow is simple:

### Step 1: Write a PRD

This is the most important step. Take an hour or two. Write down what you actually want to build - the problem, the constraints, the expected outcomes. This becomes your source of truth. Work with an AI tool to build it. It should be a collaboration between product teams, engineering teams.

```markdown
# Weather Dashboard

A simple web app showing current weather for a given city.

Users enter a city name and see temperature, conditions, and a 5-day forecast.
Use the OpenWeather API. UI should be clean and mobile-friendly.
```

### Step 2: Import and Generate Tasks

```bash
afk import requirements.md
```

afk uses your AI CLI to analyse the PRD and break it into small, AI-sized tasks. The output goes to `.afk/tasks.json`.

### Step 3: Review the Tasks

This step is critical. Run `afk tasks` and actually read what it generated. Are the tasks the right size? Do they make sense? Is anything missing or misunderstood?

If the tasks don't look right, you have two options: refine the PRD and re-import, or edit `.afk/tasks.json` directly. Either way, don't skip this step. The quality of the tasks determines the quality of the output. Garbage in, garbage out.

### Step 4: Run the Loop

```bash
afk go        # Start the autonomous loop
```

afk works through the tasks one by one. Each iteration spawns a **fresh AI instance** with clean context. It implements the task, runs quality gates (lint, test, typecheck), and auto-commits if everything passes. If something fails, it captures learnings and loops.

### How Memory Persists

The Ralph approach isn't about starting from *nothing* each time. It's about starting with the *right* context. afk maintains memory through:

- **Git history** - Commits from previous iterations
- **progress.json** - Task status and per-task learnings (short-term memory)  
- **AGENTS.md** files - Project-wide patterns and conventions (long-term memory)

Crucially, this isn't about piling up endless learnings until they become a novel. It's about keeping AGENTS.md **fresh and focused** - capturing only the key insights that will help the next iteration succeed. The AI is explicitly instructed to curate, not accumulate.

## A Different Kind of Away

The name "afk" isn't just cute - it's the point.

When you step away from keyboard, you're not abandoning the work. You're giving the AI space to operate without your interference. And you're giving *yourself* space to think strategically - to talk to product, understand the problem properly, or just take a walk.

That separation matters. When you come back, you read the output cold, like a code review. You're less attached to it. You spot gaps faster. Your judgement improves.

## The Pace of Change

What's clear right now is that things are moving fast. AI coding tools are evolving by the month, sometimes by the week. What works today might look different tomorrow.

Even Linus Torvalds - not exactly known for jumping on hype trains - is [experimenting with vibe coding](https://www.theregister.com/2026/01/13/linus_torvalds_vibe_coding/). As he put it at the Open Source Summit Asia: it's fine for projects that don't really matter. For his guitar pedal visualiser, that's the sweet spot.

The Ralph approach sits somewhere in between pure vibe coding and the traditional interactive chat. It's structured enough for serious work, but hands-off enough to let AI do what it's good at.

## Try It Out

afk is still early. I'm pleased with where it is now, but there will be rough edges. That's fine - I'm experimenting in the open and learning as I go.

If this sounds interesting, give it a try:

### Install

```bash
# macOS / Linux
curl -fsSL https://raw.githubusercontent.com/m0nkmaster/afk/main/scripts/install.sh | bash

# Windows (PowerShell)
irm https://raw.githubusercontent.com/m0nkmaster/afk/main/scripts/install.ps1 | iex
```

### Get Started

Collaborate, speak to people, write a PRD you're happy with.

```bash
afk import requirements.md    # Parse your PRD into tasks
afk tasks                     # Review - refine PRD or edit tasks if needed
afk go                        # Start the autonomous loop
```

Pick something real. Preferably something sizeable. Write clear requirements. Review the tasks - don't skip this. Let it run. Step away.

That's where this approach starts to click.

---

To find out more and dive into the detail, take a look on GitHub.

**Repo:** [github.com/m0nkmaster/afk](https://github.com/m0nkmaster/afk)

I'd love feedback - especially from people already feeling the limits of chat-based workflows. If something feels awkward, that's useful signal. If something clicks, I want to know why.

Contributions welcome. Stay curious. Embrace the tools.

*- Rob MacDonald*
