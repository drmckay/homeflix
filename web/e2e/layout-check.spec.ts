import { test, expect } from '@playwright/test';

const baseUrl = process.env.PLAYWRIGHT_BASE_URL ?? 'http://localhost:5173';

test('verify card overlaps next row on hover', async ({ page }) => {
    await page.goto(`${baseUrl}/`, { waitUntil: 'networkidle' });
    await page.waitForTimeout(1000);

    // Get Continue Watching section
    const continueWatching = page.locator('section[aria-label="Continue Watching section"]');

    // Get first card in Continue Watching
    const firstCard = continueWatching.locator('.movie-card').first();

    // Hover over the card and wait for animation
    await firstCard.hover();
    await page.waitForTimeout(600); // Wait for 300ms delay + 300ms animation

    // Take screenshot showing the overlap
    await page.screenshot({ path: '/tmp/overlap-final.png', fullPage: true });

    // Get card content bounds after hover
    const cardContent = firstCard.locator('.movie-card-content');
    const contentBounds = await cardContent.boundingBox();
    console.log('Hovered card bounds:', contentBounds);

    // Get Recently Added section bounds
    const recentlyAdded = page.locator('section[aria-label="Recently Added section"]');
    const recentlyAddedBounds = await recentlyAdded.boundingBox();
    console.log('Recently Added bounds:', recentlyAddedBounds);

    if (contentBounds && recentlyAddedBounds) {
        const cardBottom = contentBounds.y + contentBounds.height;
        const nextRowTop = recentlyAddedBounds.y;
        const overlap = cardBottom - nextRowTop;
        console.log(`Card bottom: ${cardBottom}px`);
        console.log(`Next row top: ${nextRowTop}px`);
        console.log(`Overlap: ${overlap}px (positive = overlapping)`);

        // Take closeup of the overlap area
        await page.screenshot({
            path: '/tmp/overlap-closeup.png',
            clip: {
                x: 0,
                y: Math.max(0, contentBounds.y - 20),
                width: 500,
                height: contentBounds.height + 100
            }
        });
    }

    // Check z-indexes
    const sectionZIndex = await continueWatching.evaluate(el => window.getComputedStyle(el).zIndex);
    const cardZIndex = await cardContent.evaluate(el => window.getComputedStyle(el).zIndex);
    console.log('Section z-index:', sectionZIndex);
    console.log('Card z-index:', cardZIndex);
});
