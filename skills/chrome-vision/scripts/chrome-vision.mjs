#!/usr/bin/env node
import readline from "node:readline"; import { ChromeVisionBroker } from "../src/broker.mjs"; import { loadChromeVisionConfig } from "../src/config.mjs";
const configPath = process.argv[2]; if (!configPath) throw new Error("Usage: chrome-vision <config.yaml|config.json>");
const { config } = await loadChromeVisionConfig(configPath);
const broker = new ChromeVisionBroker(config);
const lines = readline.createInterface({ input: process.stdin, crlfDelay: Infinity });
for await (const line of lines) { if (!line.trim()) continue; try { console.log(JSON.stringify(await broker.request(JSON.parse(line)))); } catch (error) { console.log(JSON.stringify({ error: error.message })); } }
