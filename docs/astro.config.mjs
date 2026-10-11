import { defineConfig } from 'astro/config';
import starlight from '@astrojs/starlight';
import mermaid from 'astro-mermaid';
import starlightAurora from 'starlight-aurora';
import mdbookRedirects from 'starlight-aurora/mdbook-redirects';

// The docs workflow sets DOCS_SITE and DOCS_BASE on main. On a pull request
// and locally they are empty, and the site is at /.
const site = process.env.DOCS_SITE || undefined;
const base = process.env.DOCS_BASE || '/';

const PROJECT = 'Squire';
const REPO = 'https://github.com/colliery-io/squire';

export default defineConfig({
	site,
	base,
	// Mermaid is one large chunk, loaded only on a page with a diagram.
	vite: { build: { chunkSizeWarningLimit: 4096 } },
	integrations: [
		// Before Starlight. Light diagrams in the light theme; dark diagrams in
		// the dark theme and in hyper (the default theme applies to every theme
		// that astro-mermaid does not know).
		mermaid({ theme: 'dark', autoTheme: true }),
		// The old mdBook addresses (*.html) send the reader to the new pages.
		mdbookRedirects(),
		starlight({
			title: PROJECT,
			description:
				'Squire, a self-hosted chores and allowance app for families. Docs for households and self-hosters.',
			social: [{ icon: 'github', label: 'GitHub', href: REPO }],
			editLink: { baseUrl: `${REPO}/edit/main/docs/` },
			plugins: [starlightAurora({ hyper: true })],
			// In the order of the old SUMMARY.md.
			sidebar: [
				{ label: 'Introduction', slug: 'index' },
				{
					label: 'Tutorials',
					items: ['tutorials/family-setup', 'tutorials/self-host-setup'],
				},
				{
					label: 'How-to guides',
					items: [
						{
							label: 'For families',
							items: [
								'how-to/install-app',
								'how-to/pair-phone',
								'how-to/chores-and-rewards',
								'how-to/cash-out',
							],
						},
						{
							label: 'For self-hosters',
							items: [
								'how-to/run-server',
								'how-to/docker-tailscale',
								'how-to/backup-restore',
							],
						},
					],
				},
				{
					label: 'Reference',
					items: ['reference/configuration', 'reference/glossary'],
				},
				{
					label: 'Explanation',
					items: ['explanation/how-squire-works', 'explanation/delivery-and-updates'],
				},
			],
		}),
	],
});
