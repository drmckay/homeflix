//! Collection Manager
//!
//! Service for managing media collections.
//! Handles collection detection, creation, and linking.

use std::sync::Arc;
use std::collections::HashMap;
use tracing::{info, debug, warn};

use crate::domain::entities::{Collection, CollectionItem};
use crate::domain::repositories::{MediaRepository, SeriesRepository, CollectionRepository};
use crate::domain::presets::PresetCollection;
use crate::domain::events::{
    CollectionCreatedEvent,
    CollectionUpdatedEvent,
    CollectionItemAddedEvent,
};
use crate::interfaces::external_services::TmdbService;
use crate::interfaces::messaging::EventBus;
use crate::shared::error::ApplicationError;

/// Collection Manager
///
/// Manages media collections:
/// 1. Detects collections from TMDB metadata
/// 2. Creates preset franchise collections
/// 3. Links media to collections
/// 4. Updates collection statistics
///
/// # Architecture Notes
/// - Uses dependency injection for all services
/// - Batch processing for efficiency
/// - Handles collection merging
pub struct CollectionManager<E: EventBus + ?Sized = crate::infrastructure::messaging::InMemoryEventBus> {
    /// Media repository for fetching media
    media_repository: Arc<dyn MediaRepository>,
    /// Series repository for fetching series
    series_repository: Arc<dyn SeriesRepository>,
    /// Collection repository for persistence
    collection_repository: Arc<dyn CollectionRepository>,
    /// TMDB service for fetching collection metadata
    tmdb_service: Arc<dyn TmdbService>,
    /// Event bus for publishing events
    event_bus: Arc<E>,
}

impl<E: EventBus + ?Sized> CollectionManager<E> {
    /// Creates a new collection manager
    ///
    /// # Arguments
    /// * `media_repository` - Repository for media persistence
    /// * `series_repository` - Repository for series data
    /// * `collection_repository` - Repository for collection data
    /// * `tmdb_service` - TMDB service for metadata lookup
    /// * `event_bus` - Event bus for publishing events
    pub fn new(
        media_repository: Arc<dyn MediaRepository>,
        series_repository: Arc<dyn SeriesRepository>,
        collection_repository: Arc<dyn CollectionRepository>,
        tmdb_service: Arc<dyn TmdbService>,
        event_bus: Arc<E>,
    ) -> Self {
        Self {
            media_repository,
            series_repository,
            collection_repository,
            tmdb_service,
            event_bus,
        }
    }

    /// Detects and creates collections from TMDB metadata
    ///
    /// This method:
    /// 1. Fetches all movies with TMDB IDs
    /// 2. For each movie, fetches TMDB details to get collection info
    /// 3. Creates or updates collections accordingly
    /// 4. Tracks which movies belong to which collections
    ///
    /// # Returns
    /// * `Result<CollectionStats, ApplicationError>` - Collection statistics
    ///
    /// # Errors
    /// Returns error if:
    /// - Media lookup fails
    /// - Collection creation fails
    pub async fn detect_and_create_collections(&self) -> Result<CollectionStats, ApplicationError> {
        info!("Starting collection detection from TMDB metadata");

        // Get all movies with TMDB IDs
        let movies = self.media_repository
            .find_by_type(crate::domain::value_objects::MediaType::Movie)
            .await?;

        // Track collections and their movies
        // Key: TMDB collection ID, Value: (collection_info, list of movie IDs)
        let mut collection_map: HashMap<i64, (crate::interfaces::external_services::CollectionInfo, Vec<i64>)> = HashMap::new();

        // Fetch TMDB details for each movie to get collection info
        for movie in &movies {
            let tmdb_id = match movie.tmdb_id {
                Some(id) => id,
                None => continue, // Skip movies without TMDB ID
            };

            let movie_id = match movie.id {
                Some(id) => id,
                None => continue,
            };

            // Fetch TMDB details
            let details = match self.tmdb_service.fetch_movie_details(tmdb_id).await {
                Ok(Some(d)) => d,
                Ok(None) => {
                    debug!("No TMDB details found for movie {} (TMDB {})", movie.title, tmdb_id);
                    continue;
                }
                Err(e) => {
                    debug!("Failed to fetch TMDB details for '{}' (TMDB {}): {}", movie.title, tmdb_id, e);
                    continue;
                }
            };

            // Check if movie belongs to a collection
            if let Some(collection_info) = details.belongs_to_collection {
                debug!(
                    "Movie '{}' belongs to collection '{}' (TMDB {})",
                    movie.title, collection_info.name, collection_info.id
                );

                collection_map
                    .entry(collection_info.id)
                    .or_insert_with(|| (collection_info.clone(), Vec::new()))
                    .1
                    .push(movie_id);
            }
        }

        let mut created_count = 0;
        let mut updated_count = 0;
        let mut linked_count = 0;
        let mut skipped_for_preset = 0;

        // Get preset names/keywords to avoid creating duplicate auto collections
        // These franchises have curated preset timelines that should take precedence
        let preset_keywords: Vec<&str> = vec![
            "stargate", "star trek", "marvel", "mcu", "avengers",
        ];

        // Build a lookup map of movies by TMDB ID for quick availability checks
        let movie_by_tmdb_id: HashMap<i64, &_> = movies
            .iter()
            .filter_map(|m| m.tmdb_id.map(|tid| (tid, m)))
            .collect();

        // Create or update collections
        for (tmdb_collection_id, (collection_info, movie_ids)) in &collection_map {
            // Skip collections that overlap with our curated presets
            // These franchises have manually curated timelines with correct ordering
            let collection_name_lower = collection_info.name.to_lowercase();
            let is_preset_covered = preset_keywords.iter().any(|kw| collection_name_lower.contains(kw));

            if is_preset_covered {
                debug!(
                    "Skipping TMDB collection '{}' - covered by preset timeline",
                    collection_info.name
                );
                skipped_for_preset += 1;
                continue;
            }

            // Fetch full collection details from TMDB (we need the parts list for items)
            let tmdb_details = match self.tmdb_service.fetch_collection_details(*tmdb_collection_id).await {
                Ok(Some(details)) => Some(details),
                Ok(None) => {
                    debug!("Could not fetch collection details for TMDB {}", tmdb_collection_id);
                    None
                }
                Err(e) => {
                    debug!("Failed to fetch collection details for TMDB {}: {}", tmdb_collection_id, e);
                    None
                }
            };

            // Check if collection already exists
            let collection_id = match self.collection_repository.find_by_tmdb_id(*tmdb_collection_id).await {
                Ok(Some(mut existing)) => {
                    // Update collection if needed
                    let mut needs_update = false;

                    // Update poster if missing
                    if existing.poster_url.is_none() && collection_info.poster_path.is_some() {
                        existing.poster_url = collection_info.poster_path.as_ref()
                            .map(|p| format!("https://image.tmdb.org/t/p/w500{}", p));
                        needs_update = true;
                    }

                    // Update backdrop if missing
                    if existing.backdrop_url.is_none() && collection_info.backdrop_path.is_some() {
                        existing.backdrop_url = collection_info.backdrop_path.as_ref()
                            .map(|b| format!("https://image.tmdb.org/t/p/w1280{}", b));
                        needs_update = true;
                    }

                    // Update available items count
                    let new_available = movie_ids.len() as i32;
                    if existing.available_items != new_available {
                        existing.available_items = new_available;
                        needs_update = true;
                    }

                    // Fix collections with total_items=0 by fetching from TMDB
                    if existing.total_items == 0 && new_available > 0 {
                        if let Some(ref details) = tmdb_details {
                            existing.total_items = details.total_parts;
                            debug!("Fixed total_items for '{}': {} (from TMDB)", existing.name, details.total_parts);
                        } else {
                            existing.total_items = new_available;
                        }
                        needs_update = true;
                    }

                    if needs_update {
                        if let Err(e) = self.collection_repository.update(&existing).await {
                            warn!("Failed to update collection '{}': {}", existing.name, e);
                        } else {
                            debug!("Updated collection '{}' with {} available items", existing.name, new_available);
                            
                            // Publish collection updated event
                            let event = CollectionUpdatedEvent::new(
                                existing.id.unwrap_or(0),
                                existing.name.clone(),
                                existing.total_items,
                                existing.available_items,
                            );
                            if let Err(e) = self.event_bus.publish(event).await {
                                warn!("Failed to publish collection updated event: {}", e);
                            }
                            
                            updated_count += 1;
                        }
                    }

                    linked_count += movie_ids.len();
                    existing.id.unwrap_or(0)
                }
                Ok(None) => {
                    // Determine total items and images from TMDB details
                    let (total_items, poster_url, backdrop_url) = if let Some(ref details) = tmdb_details {
                        debug!(
                            "TMDB collection '{}' has {} total parts",
                            details.name, details.total_parts
                        );
                        (
                            details.total_parts,
                            details.poster_path.clone().map(|p| format!("https://image.tmdb.org/t/p/w500{}", p)),
                            details.backdrop_path.clone().map(|b| format!("https://image.tmdb.org/t/p/w1280{}", b)),
                        )
                    } else {
                        (
                            movie_ids.len() as i32,
                            collection_info.poster_path.as_ref().map(|p| format!("https://image.tmdb.org/t/p/w500{}", p)),
                            collection_info.backdrop_path.as_ref().map(|b| format!("https://image.tmdb.org/t/p/w1280{}", b)),
                        )
                    };

                    // Create new collection with accurate total count
                    let mut collection = Collection::new(collection_info.name.clone())?
                        .with_tmdb_collection_id(Some(*tmdb_collection_id))
                        .with_poster_url(poster_url)
                        .with_backdrop_url(backdrop_url)
                        .with_collection_type("auto".to_string());

                    collection.available_items = movie_ids.len() as i32;
                    collection.total_items = total_items;

                    match self.collection_repository.save(&collection).await {
                        Ok(id) => {
                            info!(
                                "Created collection '{}' (TMDB {}) with ID {} - {}/{} movies available",
                                collection_info.name, tmdb_collection_id, id, movie_ids.len(), total_items
                            );
                            
                            // Publish collection created event
                            let event = CollectionCreatedEvent::new(
                                id,
                                collection_info.name.clone(),
                                Some(*tmdb_collection_id),
                                "auto".to_string(),
                            );
                            if let Err(e) = self.event_bus.publish(event).await {
                                warn!("Failed to publish collection created event: {}", e);
                            }
                            
                            created_count += 1;
                            linked_count += movie_ids.len();
                            id
                        }
                        Err(e) => {
                            warn!("Failed to create collection '{}': {}", collection_info.name, e);
                            continue;
                        }
                    }
                }
                Err(e) => {
                    warn!("Error checking for existing collection {}: {}", tmdb_collection_id, e);
                    continue;
                }
            };

            // Now create/update collection items from TMDB parts
            if let Some(details) = tmdb_details {
                // Get existing items to avoid duplicates
                let existing_items = match self.collection_repository.find_items(collection_id).await {
                    Ok(items) => items,
                    Err(e) => {
                        warn!("Failed to get existing items for collection {}: {}", collection_id, e);
                        Vec::new()
                    }
                };
                let existing_by_tmdb: HashMap<i64, &CollectionItem> = existing_items
                    .iter()
                    .map(|item| (item.tmdb_id, item))
                    .collect();

                for (idx, part) in details.parts.iter().enumerate() {
                    // Check if this movie is available in our library
                    let (is_available, media_id) = match movie_by_tmdb_id.get(&part.tmdb_id) {
                        Some(movie) => (true, movie.id),
                        None => (false, None),
                    };

                    if let Some(existing_item) = existing_by_tmdb.get(&part.tmdb_id) {
                        // Update existing item if availability changed
                        if existing_item.is_available != is_available || existing_item.media_id != media_id {
                            let mut updated_item = (*existing_item).clone();
                            updated_item.is_available = is_available;
                            updated_item.media_id = media_id;
                            if let Err(e) = self.collection_repository.update_item(&updated_item).await {
                                warn!("Failed to update collection item '{}': {}", part.title, e);
                            }
                        }
                    } else {
                        // Create new collection item
                        let item = CollectionItem {
                            id: 0, // Will be assigned by database
                            collection_id,
                            tmdb_id: part.tmdb_id,
                            media_type: "movie".to_string(),
                            title: part.title.clone(),
                            overview: part.overview.clone(),
                            poster_url: part.poster_path.as_ref().map(|p| format!("https://image.tmdb.org/t/p/w500{}", p)),
                            release_date: part.release_date.clone(),
                            timeline_order: (idx + 1) as i32, // Use index as order
                            release_order: (idx + 1) as i32,
                            timeline_year: part.release_date.as_ref()
                                .and_then(|d| d.split('-').next())
                                .and_then(|y| y.parse().ok()),
                            timeline_notes: None,
                            is_available,
                            media_id,
                        };

                        match self.collection_repository.save_item(&item).await {
                            Ok(_) => {
                                debug!("Added item '{}' to collection (available: {})", part.title, is_available);
                                
                                // Publish collection item added event
                                let event = CollectionItemAddedEvent::new(
                                    collection_id,
                                    media_id,
                                    part.tmdb_id,
                                    "movie".to_string(),
                                    part.title.clone(),
                                );
                                if let Err(e) = self.event_bus.publish(event).await {
                                    warn!("Failed to publish collection item added event: {}", e);
                                }
                            }
                            Err(e) => {
                                warn!("Failed to save collection item '{}': {}", part.title, e);
                            }
                        }
                    }
                }
            }
        }

        let stats = CollectionStats {
            total_collections: created_count,
            total_media_linked: linked_count,
        };

        if created_count > 0 || updated_count > 0 || skipped_for_preset > 0 {
            info!(
                "Collection detection complete: {} created, {} updated, {} media linked, {} skipped (preset coverage)",
                created_count, updated_count, linked_count, skipped_for_preset
            );
        }

        Ok(stats)
    }

    /// Creates a custom collection
    ///
    /// # Arguments
    /// * `name` - Collection name
    /// * `media_ids` - List of media IDs to include
    ///
    /// # Returns
    /// * `Result<i64, ApplicationError>` - Collection ID
    pub async fn create_custom_collection(
        &self,
        name: String,
        media_ids: Vec<i64>,
    ) -> Result<i64, ApplicationError> {
        info!("Creating custom collection: {}", name);

        let mut collection = Collection::new(name.clone())?
            .with_collection_type("custom".to_string());
        collection.available_items = media_ids.len() as i32;

        let collection_id = self.collection_repository.save(&collection).await?;

        // Publish collection created event
        let event = CollectionCreatedEvent::new(
            collection_id,
            name.clone(),
            None, // Custom collections don't have TMDB ID
            "manual".to_string(),
        );
        if let Err(e) = self.event_bus.publish(event).await {
            warn!("Failed to publish collection created event: {}", e);
        }

        info!(
            "Custom collection created: {} (ID: {}) with {} media",
            name, collection_id, media_ids.len()
        );

        Ok(collection_id)
    }

    /// Lists all collections
    ///
    /// # Returns
    /// * `Result<Vec<Collection>, ApplicationError>` - List of collections
    pub async fn list_collections(&self) -> Result<Vec<Collection>, ApplicationError> {
        debug!("Listing all collections");
        let collections = self.collection_repository.find_all().await?;
        debug!("Found {} collections", collections.len());
        Ok(collections)
    }

    /// Gets collection by ID with details
    ///
    /// # Arguments
    /// * `collection_id` - Collection ID
    ///
    /// # Returns
    /// * `Result<CollectionDetail, ApplicationError>` - Collection with details
    pub async fn get_collection(&self, collection_id: i64) -> Result<CollectionDetail, ApplicationError> {
        let collection = self.collection_repository
            .find_by_id(collection_id)
            .await?
            .ok_or_else(|| ApplicationError::Domain(
                crate::shared::error::DomainError::NotFound(format!("Collection {} not found", collection_id))
            ))?;

        Ok(CollectionDetail {
            collection,
            media_count: 0, // Would need collection_items table query
        })
    }

    /// Deletes a collection
    ///
    /// # Arguments
    /// * `collection_id` - Collection ID to delete
    pub async fn delete_collection(&self, collection_id: i64) -> Result<(), ApplicationError> {
        info!("Deleting collection ID: {}", collection_id);
        self.collection_repository.delete(collection_id).await?;
        info!("Collection {} deleted", collection_id);
        Ok(())
    }

    /// Creates and updates preset franchise collections
    ///
    /// This method:
    /// 1. Gets all preset collection definitions (Star Trek, Stargate, MCU, etc.)
    /// 2. Checks which items are available in the library
    /// 3. Only creates collections if at least one item is available
    /// 4. Creates collection items only for available media
    /// 5. Updates availability status and counts
    ///
    /// # Arguments
    /// * `presets` - Vector of preset collections to create/update
    pub async fn create_preset_collections(&self, presets: Vec<PresetCollection>) -> Result<PresetStats, ApplicationError> {
        info!("Creating/updating preset franchise collections");
        let mut created_collections = 0;
        let mut total_items = 0;
        let mut available_items = 0;

        // Build lookup maps for matching
        // Movies by TMDB ID
        let movies = self.media_repository
            .find_by_type(crate::domain::value_objects::MediaType::Movie)
            .await?;
        let movie_by_tmdb: HashMap<i64, _> = movies
            .iter()
            .filter_map(|m| m.tmdb_id.map(|tid| (tid, m)))
            .collect();

        // Series by TMDB ID
        let all_series = self.series_repository.find_all().await?;
        let series_by_tmdb: HashMap<i64, _> = all_series
            .iter()
            .filter_map(|s| s.tmdb_id.map(|tid| (tid, s)))
            .collect();

        for preset in presets {
            // First pass: count how many items are available in library
            let mut items_to_create: Vec<(_, bool, Option<i64>)> = Vec::new();
            let mut collection_available_count = 0;

            for (idx, preset_item) in preset.items.iter().enumerate() {
                // Check availability in library
                let (is_available, media_id) = if preset_item.media_type == "movie" {
                    match movie_by_tmdb.get(&preset_item.tmdb_id) {
                        Some(movie) => (true, movie.id),
                        None => (false, None),
                    }
                } else {
                    // TV series
                    match series_by_tmdb.get(&preset_item.tmdb_id) {
                        Some(series) => (true, series.id),
                        None => (false, None),
                    }
                };

                if is_available {
                    collection_available_count += 1;
                }

                items_to_create.push((idx, is_available, media_id));
            }

            // Skip this collection if no items are available in library
            if collection_available_count == 0 {
                debug!("Skipping preset '{}': no items available in library", preset.name);
                continue;
            }

            total_items += preset.items.len();
            available_items += collection_available_count;

            // Fetch TMDB metadata if preset has a TMDB collection ID
            let (tmdb_name, tmdb_poster, tmdb_backdrop) = if let Some(tmdb_id) = preset.tmdb_collection_id {
                match self.tmdb_service.fetch_collection_details(tmdb_id).await {
                    Ok(Some(details)) => {
                        debug!("Fetched TMDB metadata for preset '{}': name='{}' from TMDB {}",
                            preset.name, details.name, tmdb_id);
                        (
                            Some(details.name),
                            details.poster_path.map(|p| format!("https://image.tmdb.org/t/p/w500{}", p)),
                            details.backdrop_path.map(|b| format!("https://image.tmdb.org/t/p/w1280{}", b)),
                        )
                    }
                    Ok(None) => {
                        debug!("No TMDB details found for collection ID {}", tmdb_id);
                        (None, None, None)
                    }
                    Err(e) => {
                        debug!("Failed to fetch TMDB collection details for {}: {}", tmdb_id, e);
                        (None, None, None)
                    }
                }
            } else {
                (None, None, None)
            };

            // Use TMDB name if available, otherwise use preset name
            let collection_name = tmdb_name.as_deref().unwrap_or(&preset.name);

            // Check if collection already exists (by TMDB ID first, then by name)
            let existing_collection = if let Some(tmdb_id) = preset.tmdb_collection_id {
                // First check by TMDB ID
                let by_tmdb = self.collection_repository.find_by_tmdb_id(tmdb_id).await?;
                if by_tmdb.is_some() {
                    by_tmdb
                } else {
                    // Also check by preset name (might have been created before TMDB integration)
                    self.collection_repository.find_by_name(&preset.name).await?
                }
            } else {
                self.collection_repository.find_by_name(&preset.name).await?
            };

            let collection_id = match existing_collection {
                Some(mut existing) => {
                    debug!("Preset collection '{}' already exists (ID: {})", preset.name, existing.id.unwrap_or(0));

                    // Update with TMDB metadata if we have it and it's missing/different
                    let mut needs_update = false;

                    if let Some(ref name) = tmdb_name {
                        if existing.name != *name {
                            existing.name = name.clone();
                            needs_update = true;
                        }
                    }

                    if tmdb_poster.is_some() && existing.poster_url != tmdb_poster {
                        existing.poster_url = tmdb_poster.clone();
                        needs_update = true;
                    }

                    if tmdb_backdrop.is_some() && existing.backdrop_url != tmdb_backdrop {
                        existing.backdrop_url = tmdb_backdrop.clone();
                        needs_update = true;
                    }

                    // Update TMDB collection ID if not set
                    if preset.tmdb_collection_id.is_some() && existing.tmdb_collection_id != preset.tmdb_collection_id {
                        existing.tmdb_collection_id = preset.tmdb_collection_id;
                        needs_update = true;
                    }

                    // Ensure collection_type is preset (in case it was auto-detected before)
                    if existing.collection_type != "preset" {
                        existing.collection_type = "preset".to_string();
                        needs_update = true;
                    }

                    if needs_update {
                    if let Err(e) = self.collection_repository.update(&existing).await {
                        warn!("Failed to update collection '{}': {}", existing.name, e);
                    } else {
                        debug!("Updated collection '{}' with TMDB metadata", existing.name);
                        
                        // Publish collection updated event
                        let event = CollectionUpdatedEvent::new(
                            existing.id.unwrap_or(0),
                            existing.name.clone(),
                            existing.total_items,
                            existing.available_items,
                        );
                        if let Err(e) = self.event_bus.publish(event).await {
                            warn!("Failed to publish collection updated event: {}", e);
                        }
                    }
                    }

                    existing.id.unwrap_or(0)
                }
                None => {
                    // Create new preset collection with TMDB metadata
                    let collection = Collection::new(collection_name.to_string())?
                        .with_description(Some(preset.description.to_string()))
                        .with_tmdb_collection_id(preset.tmdb_collection_id)
                        .with_poster_url(tmdb_poster)
                        .with_backdrop_url(tmdb_backdrop)
                        .with_collection_type("preset".to_string())
                        .with_sort_mode("timeline".to_string());

                    let id = self.collection_repository.save(&collection).await?;
                    
                    // Publish collection created event
                    let event = CollectionCreatedEvent::new(
                        id,
                        collection_name.to_string(),
                        None, // Preset collections don't have TMDB ID
                        "preset".to_string(),
                    );
                    if let Err(e) = self.event_bus.publish(event).await {
                        warn!("Failed to publish collection created event: {}", e);
                    }
                    
                    info!("Created preset collection '{}' (ID: {}) - {}/{} items available",
                        collection_name, id, collection_available_count, preset.items.len());
                    created_collections += 1;
                    id
                }
            };

            // Get existing items for this collection
            // Key includes timeline_order to handle same show appearing multiple times (e.g., SG-1 S1-8 and S9-10)
            let existing_items = self.collection_repository.find_items(collection_id).await?;
            let existing_by_key: HashMap<(i64, String, i32), &CollectionItem> = existing_items
                .iter()
                .map(|item| ((item.tmdb_id, item.media_type.clone(), item.timeline_order), item))
                .collect();

            // Process each preset item
            for (idx, is_available, media_id) in items_to_create {
                let preset_item = &preset.items[idx];
                let key = (preset_item.tmdb_id, preset_item.media_type.clone(), preset_item.timeline_order);

                if let Some(existing_item) = existing_by_key.get(&key) {
                    // Check if we need to update availability or fill in missing metadata
                    let needs_availability_update = existing_item.is_available != is_available || existing_item.media_id != media_id;
                    let needs_metadata = existing_item.poster_url.is_none() || existing_item.overview.is_none();

                    if needs_availability_update || needs_metadata {
                        let mut updated_item = (*existing_item).clone();
                        updated_item.is_available = is_available;
                        updated_item.media_id = media_id;

                        // Fetch missing metadata if needed
                        if needs_metadata {
                            let (overview, poster_url, release_date) = if preset_item.media_type == "movie" {
                                match self.tmdb_service.fetch_movie_details(preset_item.tmdb_id).await {
                                    Ok(Some(details)) => (
                                        Some(details.overview),
                                        details.poster_path.map(|p| format!("https://image.tmdb.org/t/p/w500{}", p)),
                                        Some(details.release_date),
                                    ),
                                    _ => (None, None, None),
                                }
                            } else {
                                match self.tmdb_service.fetch_tv_details(preset_item.tmdb_id).await {
                                    Ok(Some(details)) => (
                                        Some(details.overview),
                                        details.poster_path.map(|p| format!("https://image.tmdb.org/t/p/w500{}", p)),
                                        Some(details.first_air_date),
                                    ),
                                    _ => (None, None, None),
                                }
                            };

                            if overview.is_some() && updated_item.overview.is_none() {
                                updated_item.overview = overview;
                            }
                            if poster_url.is_some() && updated_item.poster_url.is_none() {
                                updated_item.poster_url = poster_url;
                            }
                            if release_date.is_some() && updated_item.release_date.is_none() {
                                updated_item.release_date = release_date;
                            }
                        }

                        if let Err(e) = self.collection_repository.update_item(&updated_item).await {
                            warn!("Failed to update collection item '{}': {}", preset_item.title, e);
                        } else {
                            debug!("Updated item '{}' (availability: {}, metadata: {})",
                                preset_item.title, is_available, updated_item.poster_url.is_some());
                        }
                    }
                } else {
                    // Fetch TMDB metadata for this item (poster, overview, release_date)
                    let (overview, poster_url, release_date) = if preset_item.media_type == "movie" {
                        match self.tmdb_service.fetch_movie_details(preset_item.tmdb_id).await {
                            Ok(Some(details)) => (
                                Some(details.overview),
                                details.poster_path.map(|p| format!("https://image.tmdb.org/t/p/w500{}", p)),
                                Some(details.release_date),
                            ),
                            Ok(None) => {
                                debug!("No TMDB movie details for {} ({})", preset_item.title, preset_item.tmdb_id);
                                (None, None, None)
                            }
                            Err(e) => {
                                debug!("Failed to fetch TMDB movie {}: {}", preset_item.tmdb_id, e);
                                (None, None, None)
                            }
                        }
                    } else {
                        // TV show
                        match self.tmdb_service.fetch_tv_details(preset_item.tmdb_id).await {
                            Ok(Some(details)) => (
                                Some(details.overview),
                                details.poster_path.map(|p| format!("https://image.tmdb.org/t/p/w500{}", p)),
                                Some(details.first_air_date),
                            ),
                            Ok(None) => {
                                debug!("No TMDB TV details for {} ({})", preset_item.title, preset_item.tmdb_id);
                                (None, None, None)
                            }
                            Err(e) => {
                                debug!("Failed to fetch TMDB TV {}: {}", preset_item.tmdb_id, e);
                                (None, None, None)
                            }
                        }
                    };

                    // Create new collection item with TMDB metadata
                    let item = CollectionItem {
                        id: 0, // Will be assigned by database
                        collection_id,
                        tmdb_id: preset_item.tmdb_id,
                        media_type: preset_item.media_type.to_string(),
                        title: preset_item.title.to_string(),
                        overview,
                        poster_url,
                        release_date,
                        timeline_order: preset_item.timeline_order,
                        release_order: preset_item.timeline_order, // Use timeline_order as release_order
                        timeline_year: preset_item.timeline_year,
                        timeline_notes: preset_item.timeline_notes.clone(),
                        is_available,
                        media_id,
                    };

                    match self.collection_repository.save_item(&item).await {
                        Ok(_) => {
                            debug!("Added item '{}' to collection (has_poster: {})", preset_item.title, item.poster_url.is_some());
                            
                            // Publish collection item added event
                            let event = CollectionItemAddedEvent::new(
                                collection_id,
                                media_id,
                                preset_item.tmdb_id,
                                preset_item.media_type.to_string(),
                                preset_item.title.to_string(),
                            );
                            if let Err(e) = self.event_bus.publish(event).await {
                                warn!("Failed to publish collection item added event: {}", e);
                            }
                        }
                        Err(e) => {
                            warn!("Failed to save collection item '{}': {}", preset_item.title, e);
                        }
                    }
                }
            }

            // Update collection counts
            let collection_total = preset.items.len() as i32;
            if let Err(e) = self.collection_repository.update_counts(collection_id, collection_total, collection_available_count as i32).await {
                warn!("Failed to update counts for collection '{}': {}", preset.name, e);
            } else {
                debug!("Updated collection '{}': {}/{} available", preset.name, collection_available_count, collection_total);
                
                // Publish collection updated event
                let event = CollectionUpdatedEvent::new(
                    collection_id,
                    preset.name.clone(),
                    collection_total,
                    collection_available_count as i32,
                );
                if let Err(e) = self.event_bus.publish(event).await {
                    warn!("Failed to publish collection updated event: {}", e);
                }
            }
        }

        let stats = PresetStats {
            collections_created: created_collections,
            total_items,
            available_items,
        };

        info!(
            "Preset collections complete: {} created, {}/{} items available",
            created_collections, available_items, total_items
        );

        Ok(stats)
    }
}

/// Statistics from preset collection creation
#[derive(Debug, Clone)]
pub struct PresetStats {
    /// Collections created
    pub collections_created: usize,
    /// Total items across all preset collections
    pub total_items: usize,
    /// Items available in library
    pub available_items: usize,
}

/// Statistics from collection detection
#[derive(Debug, Clone)]
pub struct CollectionStats {
    /// Total collections created
    pub total_collections: usize,
    /// Total media linked to collections
    pub total_media_linked: usize,
}

/// Collection detail with media count
#[derive(Debug, Clone)]
pub struct CollectionDetail {
    /// Collection entity
    pub collection: Collection,
    /// Number of media in collection
    pub media_count: usize,
}
