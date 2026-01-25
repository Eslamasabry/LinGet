#![allow(dead_code)]

use serde::{Deserialize, Serialize};
use std::collections::HashSet;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackageRelations {
    pub relations: Vec<Relation>,
    pub categories: std::collections::HashMap<String, CategoryInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Relation {
    pub trigger: Vec<String>,
    pub recommendations: Vec<RecommendedPackage>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecommendedPackage {
    pub name: String,
    pub description: String,
    pub category: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CategoryInfo {
    pub icon: String,
    pub label: String,
}

#[derive(Debug, Clone)]
pub struct Recommendation {
    pub name: String,
    pub description: String,
    pub category: String,
    pub category_icon: String,
    pub triggered_by: Vec<String>,
}

impl PackageRelations {
    pub fn load() -> Option<Self> {
        let json_str = include_str!("../../data/package_relations.json");
        serde_json::from_str(json_str).ok()
    }

    pub fn get_recommendations(
        &self,
        installed_packages: &[String],
        dismissed: &HashSet<String>,
        limit: usize,
    ) -> Vec<Recommendation> {
        let installed_set: HashSet<&str> = installed_packages.iter().map(|s| s.as_str()).collect();
        let mut recommendations = Vec::new();
        let mut seen_names: HashSet<String> = HashSet::new();

        for relation in &self.relations {
            let matching_triggers: Vec<String> = relation
                .trigger
                .iter()
                .filter(|t| installed_set.contains(t.as_str()))
                .cloned()
                .collect();

            if matching_triggers.is_empty() {
                continue;
            }

            for rec in &relation.recommendations {
                if installed_set.contains(rec.name.as_str()) {
                    continue;
                }
                if dismissed.contains(&rec.name) {
                    continue;
                }
                if seen_names.contains(&rec.name) {
                    continue;
                }

                seen_names.insert(rec.name.clone());

                let category_icon = self
                    .categories
                    .get(&rec.category)
                    .map(|c| c.icon.clone())
                    .unwrap_or_else(|| "package-x-generic-symbolic".to_string());

                recommendations.push(Recommendation {
                    name: rec.name.clone(),
                    description: rec.description.clone(),
                    category: rec.category.clone(),
                    category_icon,
                    triggered_by: matching_triggers.clone(),
                });
            }
        }

        recommendations.truncate(limit);
        recommendations
    }

    pub fn get_recommendations_for_package(
        &self,
        package_name: &str,
        installed_packages: &[String],
        dismissed: &HashSet<String>,
        limit: usize,
    ) -> Vec<Recommendation> {
        let installed_set: HashSet<&str> = installed_packages.iter().map(|s| s.as_str()).collect();
        let mut recommendations = Vec::new();

        for relation in &self.relations {
            if !relation.trigger.iter().any(|t| t == package_name) {
                continue;
            }

            for rec in &relation.recommendations {
                if installed_set.contains(rec.name.as_str()) {
                    continue;
                }
                if dismissed.contains(&rec.name) {
                    continue;
                }

                let category_icon = self
                    .categories
                    .get(&rec.category)
                    .map(|c| c.icon.clone())
                    .unwrap_or_else(|| "package-x-generic-symbolic".to_string());

                recommendations.push(Recommendation {
                    name: rec.name.clone(),
                    description: rec.description.clone(),
                    category: rec.category.clone(),
                    category_icon,
                    triggered_by: vec![package_name.to_string()],
                });

                if recommendations.len() >= limit {
                    return recommendations;
                }
            }
        }

        recommendations
    }
}

pub fn get_global_recommendations(
    installed_packages: &[String],
    dismissed: &HashSet<String>,
    limit: usize,
) -> Vec<Recommendation> {
    PackageRelations::load()
        .map(|r| r.get_recommendations(installed_packages, dismissed, limit))
        .unwrap_or_default()
}

pub fn get_package_recommendations(
    package_name: &str,
    installed_packages: &[String],
    dismissed: &HashSet<String>,
    limit: usize,
) -> Vec<Recommendation> {
    PackageRelations::load()
        .map(|r| {
            r.get_recommendations_for_package(package_name, installed_packages, dismissed, limit)
        })
        .unwrap_or_default()
}
