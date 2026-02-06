pub type ResourceEntry = crate::config::mods::ModEntry;

pub fn parse_resource_toml(contents: &str) -> Result<ResourceEntry, toml::de::Error> {
    crate::config::mods::parse_mod_toml(contents)
}
