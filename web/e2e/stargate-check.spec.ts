import { test, expect } from '@playwright/test';

test.describe('Stargate SG-1 Series Grouping', () => {
  // Configured URL from user request
  const BASE_URL = 'http://homeflix.home';

  test.beforeEach(async ({ page }) => {
    await page.goto(BASE_URL);
  });

  test('should display Stargate SG-1 as a single series', async ({ page }) => {
    // 1. Search for Stargate SG-1
    // Assuming there is a search bar or we can find it in the list
    // If list is flat seasons, we might see "Stargate Sg 1 S01", "Stargate Sg 1 S02"...
    // If grouped, we see "Stargate SG-1"

    // Wait for content to load
    await page.waitForSelector('.media-card, .series-card');

    // Check if we have multiple "Stargate SG-1 Sxx" entries (Old behavior)
    // or one "Stargate SG-1" entry (New behavior)
    
    // Look for a card with exact title "Stargate SG-1" (or close match)
    const seriesCard = page.locator('text=Stargate SG-1').first();
    
    // If the scanner fixed it, there should be one main entry.
    // If not, there might be "Stargate SG-1 Season 1", etc.
    
    await expect(seriesCard).toBeVisible();
    await seriesCard.click();

    // 2. Verify Detail Page
    // Should show metadata
    await expect(page.locator('h1')).toContainText('Stargate SG-1');
    
    // Check for overview/description (metadata presence)
    const overview = page.locator('.overview, p:has-text("Stargate")');
    await expect(overview).toBeVisible();
    await expect(overview).not.toBeEmpty();

    // 3. Verify Season Grouping
    // Should have season selector or tabs
    const seasonSelector = page.locator('select.season-select, .season-tabs');
    // Or just check if we can navigate to Season 10
    // If it was listed separately, this page would only show Season 1 files.
    
    // Let's assume a UI where seasons are listed.
    // We expect 10 seasons.
    await expect(page.locator('text=Season 1')).toBeVisible();
    await expect(page.locator('text=Season 10')).toBeVisible();

    // 4. Verify Movies
    // User mentioned Stargate.Movies folder.
    // They should ideally be under "Specials" or separate Movie entries.
    // If grouped under Series, maybe "Specials" or "Movies"?
    // Or if they are separate Movies, they should appear in Movies section, not mixed as seasons.
    // But verify they are not missing.
    
    // Go back to home/movies
    await page.goto(BASE_URL + '/movies');
    await expect(page.locator('text=Stargate (1994)')).toBeVisible();
    await expect(page.locator('text=Stargate: Continuum')).toBeVisible();
    await expect(page.locator('text=The Ark of Truth')).toBeVisible();
  });
});
