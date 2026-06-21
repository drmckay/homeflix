import { createReadStream, statSync } from 'node:fs';
import { createServer } from 'node:http';
import { dirname, join, normalize } from 'node:path';
import { fileURLToPath } from 'node:url';
import { handler } from './build/handler.js';

const root = dirname(fileURLToPath(import.meta.url));
const clientRoot = join(root, 'build', 'client');

const contentTypes = {
	'.br': 'application/octet-stream',
	'.css': 'text/css; charset=utf-8',
	'.gif': 'image/gif',
	'.gz': 'application/gzip',
	'.html': 'text/html; charset=utf-8',
	'.ico': 'image/x-icon',
	'.jpg': 'image/jpeg',
	'.jpeg': 'image/jpeg',
	'.js': 'application/javascript; charset=utf-8',
	'.json': 'application/json; charset=utf-8',
	'.map': 'application/json; charset=utf-8',
	'.png': 'image/png',
	'.svg': 'image/svg+xml',
	'.txt': 'text/plain; charset=utf-8',
	'.webp': 'image/webp'
};

function contentType(pathname) {
	const dot = pathname.lastIndexOf('.');
	return dot === -1 ? 'application/octet-stream' : contentTypes[pathname.slice(dot)] ?? 'application/octet-stream';
}

function getStaticFile(pathname) {
	let decoded;
	try {
		decoded = decodeURIComponent(pathname);
	} catch {
		return null;
	}

	const relative = normalize(decoded).replace(/^(\.\.[/\\])+/, '').replace(/^[/\\]+/, '');
	const file = join(clientRoot, relative);
	if (!file.startsWith(clientRoot)) return null;

	try {
		const stats = statSync(file);
		return stats.isFile() ? { file, stats } : null;
	} catch {
		return null;
	}
}

function serveStatic(req, res) {
	if (req.method !== 'GET' && req.method !== 'HEAD') return false;

	const url = new URL(req.url ?? '/', 'http://localhost');
	const match = getStaticFile(url.pathname);
	if (!match) return false;

	res.statusCode = 200;
	res.setHeader('content-type', contentType(match.file));
	res.setHeader('content-length', match.stats.size);
	res.setHeader(
		'cache-control',
		url.pathname.includes('/_app/immutable/')
			? 'public, max-age=31536000, immutable'
			: 'public, max-age=0, must-revalidate'
	);

	if (req.method === 'HEAD') {
		res.end();
		return true;
	}

	createReadStream(match.file).pipe(res);
	return true;
}

const server = createServer((req, res) => {
	if (serveStatic(req, res)) return;
	handler(req, res);
});

const host = process.env.HOST ?? '0.0.0.0';
const port = Number(process.env.PORT ?? 3000);

server.listen(port, host, () => {
	console.log(`Listening on http://${host}:${port}`);
});

for (const signal of ['SIGINT', 'SIGTERM']) {
	process.on(signal, () => {
		server.close(() => process.exit(0));
	});
}
