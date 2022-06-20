use std::collections::HashMap;

use serde::Deserialize;

use crate::error::Result;

/// Simplified entity with a lot of assumptions.
#[derive(Debug, Clone)]
pub struct SimplifiedEntity {
    pub id: String,
    pub name: String,
    pub(crate) model_name: String,
    pub(crate) iterations: usize,
    pub(crate) sound_bank: Option<String>,
    pub(crate) manifest: Option<String>,
    pub(crate) lyric_art: Option<String>,
    pub(crate) album_art_small: Option<String>,
    pub(crate) album_art_medium: Option<String>,
    pub(crate) album_art_large: Option<String>,
    pub(crate) preview_sound_bank: Option<String>,
    pub(crate) header: Option<String>,
    pub(crate) show_lights_xml_asset: Option<String>,
    pub(crate) sng_asset: Option<String>,
}

impl From<&Entity> for SimplifiedEntity {
    fn from(entity: &Entity) -> Self {
        // TODO: don't have so many memory allocations
        let properties = entity.properties_map();

        Self {
            id: entity.id.clone(),
            model_name: entity.model_name.clone(),
            name: entity.name.clone(),
            iterations: entity.iterations,
            sound_bank: properties.get("SoundBank").map(|k| k.clone()),
            manifest: properties.get("Manifest").map(|k| k.clone()),
            lyric_art: properties.get("LyricArt").map(|k| k.clone()),
            album_art_small: properties.get("AlbumArtSmall").map(|k| k.clone()),
            album_art_medium: properties.get("AlbumArtMedium").map(|k| k.clone()),
            album_art_large: properties.get("AlbumArtLarge").map(|k| k.clone()),
            preview_sound_bank: properties.get("PreviewSoundBank").map(|k| k.clone()),
            header: properties.get("Header").map(|k| k.clone()),
            show_lights_xml_asset: properties.get("ShowLightsXMLAsset").map(|k| k.clone()),
            sng_asset: properties.get("SngAsset").map(|k| k.clone()),
        }
    }
}

/// Xblock XML file representation.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename = "game", rename_all = "camelCase")]
pub struct Xblock {
    #[serde(rename = "entitySet")]
    entity_set: EntitySet,
}

impl Xblock {
    /// Parse an XML string.
    pub fn parse(xml: &str) -> Result<Self> {
        Ok(quick_xml::de::from_str(xml)?)
    }

    /// Get all entities as their simplified variant.
    pub fn simplified_entities_iter(&'_ self) -> impl Iterator<Item = SimplifiedEntity> + '_ {
        self.entity_set.entities.iter().map(|entity| entity.into())
    }
}

/// List of game entities.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EntitySet {
    #[serde(rename = "entity")]
    entities: Vec<Entity>,
}

/// Game entity, different versions of the song.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Entity {
    id: String,
    model_name: String,
    name: String,
    iterations: usize,
    properties: Properties,
}

impl Entity {
    fn properties_map(&self) -> HashMap<String, String> {
        let mut map = HashMap::new();

        for property in &self.properties.properties {
            map.insert(property.name.clone(), property.values[0].value.clone());
        }

        map
    }
}

/// List of properties of an entity.
#[derive(Debug, Clone, Deserialize)]
pub struct Properties {
    #[serde(rename = "property")]
    properties: Vec<Property>,
}

/// Property of an entity.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Property {
    name: String,

    #[serde(rename = "set")]
    values: Vec<Value>,
}

/// Value of a property.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Value {
    value: String,
}
