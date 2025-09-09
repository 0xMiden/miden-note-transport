#!/usr/bin/env node

import { createServer } from 'http';
import { readFileSync, existsSync } from 'fs';
import { join, dirname } from 'path';
import { fileURLToPath } from 'url';

const __filename = fileURLToPath(import.meta.url);
const __dirname = dirname(__filename);

const PORT = 3000;

const server = createServer((req, res) => {
    let filePath;
    
    if (req.url === '/') {
        filePath = join(__dirname, 'simple-app.html');
    } else if (req.url.startsWith('/dist/')) {
        filePath = join(__dirname, '..', 'dist', req.url.replace('/dist/', ''));
    } else if (req.url.endsWith('.html') && req.url.includes('test')) {
        // Serve test files from the test directory
        filePath = join(__dirname, '..', 'test', req.url);
    } else {
        filePath = join(__dirname, req.url);
    }
    
    if (!existsSync(filePath)) {
        res.writeHead(404, { 'Content-Type': 'text/plain' });
        res.end('File not found: ' + filePath);
        return;
    }
    
    try {
        const content = readFileSync(filePath);
        const ext = filePath.split('.').pop();
        const contentType = {
            'html': 'text/html',
            'js': 'application/javascript',
            'wasm': 'application/wasm',
            'css': 'text/css',
            'json': 'application/json'
        }[ext] || 'text/plain';
        
        res.writeHead(200, { 'Content-Type': contentType });
        res.end(content);
    } catch (error) {
        console.error('Error:', error);
        res.writeHead(500, { 'Content-Type': 'text/plain' });
        res.end('Error reading file');
    }
});

server.listen(PORT, () => {
    console.log(`Server running at http://localhost:${PORT}`);
});
