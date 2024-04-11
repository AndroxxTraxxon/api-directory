use std::fmt;

use serde::{Deserialize, Serialize};
use surrealdb::sql::Thing;
use validator::Validate;

use crate::database::{API_ROLE_TABLE, NAMESPACE_MEMBER_ROLE, ROLE_NAMESPACE_DELIMITER};

#[derive(Debug, Serialize, Deserialize, Validate, Clone)]
pub struct WebRequestApiService {
    #[validate(length(min = 3))]
    pub api_name: String,

    #[validate(length(min = 1))]
    pub version: String,

    #[validate(length(min = 3))]
    pub forward_url: String,

    pub active: bool,

    #[validate(length(min = 1))]
    pub role_namespaces: Vec<String>,

    #[validate(length(min = 1))]
    pub roles: Vec<WebApiRole>,

    #[validate(length(min = 1))]
    pub environment: String,
}

impl From<&WebRequestApiService> for Vec<WebApiRole> {
    fn from(value: &WebRequestApiService) -> Self {
        let mut roles = value.roles.clone();
        if !value.role_namespaces.is_empty() {
            value.role_namespaces.iter().for_each(|namespace| {
                roles.push(WebApiRole {
                    id: None,
                    namespace: namespace.clone(),
                    name: NAMESPACE_MEMBER_ROLE.into(),
                });
            });
        }

        roles
    }
}

#[derive(Debug, Serialize, Deserialize, Validate, Clone)]
pub struct WebResponseApiService {
    pub id: String,
    #[validate(length(min = 3))]
    pub api_name: String,

    #[validate(length(min = 3))]
    pub forward_url: String,

    pub active: bool,

    #[validate(length(min = 1))]
    pub version: String,

    #[validate(length(min = 1))]
    pub role_namespaces: Vec<String>,

    #[validate(length(min = 1))]
    pub roles: Vec<WebApiRole>,

    #[validate(length(min = 1))]
    pub environment: String,
}

#[derive(Debug, Serialize, Deserialize, Validate, Clone)]
pub struct DbFullApiService {
    pub id: Thing,
    #[validate(length(min = 3))]
    pub api_name: String,
    #[validate(length(min = 3))]
    pub forward_url: String,
    pub active: bool,
    #[validate(length(min = 1))]
    pub version: String,
    #[validate(length(min = 1))]
    pub environment: String,
    pub roles: Vec<DbApiRole>,
}

impl From<&DbFullApiService> for WebResponseApiService {
    fn from(other: &DbFullApiService) -> Self {
        let mut namespaces: Vec<String> = Vec::new();
        let mut roles: Vec<WebApiRole> = Vec::new();
        for db_role in &other.roles {
            if db_role.name == NAMESPACE_MEMBER_ROLE.to_string() {
                namespaces.push(db_role.namespace.clone());
            } else {
                roles.push(db_role.into());
            }
        }
        Self {
            id: format!("{}", other.id.id),
            api_name: other.api_name.clone(),
            forward_url: other.forward_url.clone(),
            active: other.active,
            version: other.version.clone(),
            role_namespaces: namespaces,
            roles: roles.clone(),
            environment: other.environment.clone(),
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Validate, Clone)]
pub struct DbApiServiceRequest {
    #[validate(length(min = 3))]
    pub api_name: String,

    #[validate(length(min = 3))]
    pub forward_url: String,

    pub active: bool,

    #[validate(length(min = 1))]
    pub version: String,

    #[validate(length(min = 1))]
    pub environment: String,
}

#[derive(Debug, Serialize, Deserialize, Validate, Clone)]
pub struct DbApiServiceRecord {
    pub id: Thing,
    #[validate(length(min = 3))]
    pub api_name: String,

    #[validate(length(min = 3))]
    pub forward_url: String,

    pub active: bool,

    #[validate(length(min = 1))]
    pub version: String,

    #[validate(length(min = 1))]
    pub environment: String,
}

impl From<&WebRequestApiService> for DbApiServiceRequest {
    fn from(value: &WebRequestApiService) -> Self {
        Self {
            api_name: value.api_name.clone(),
            forward_url: value.forward_url.clone(),
            active: value.active.clone(),
            version: value.version.clone(),
            environment: value.environment.clone(),
        }
    }
}

impl From<(&DbApiServiceRecord, &Vec<DbApiRole>)> for DbFullApiService {
    fn from((service, roles): (&DbApiServiceRecord, &Vec<DbApiRole>)) -> Self {
        Self {
            id: service.id.clone(),
            api_name: service.api_name.clone(),
            forward_url: service.forward_url.clone(),
            active: service.active,
            version: service.version.clone(),
            roles: roles.clone(),
            environment: service.environment.clone(),
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Validate, Clone)]
pub struct WebRequestPartialApiService {
    #[validate(length(min = 3))]
    pub api_name: Option<String>,

    #[validate(length(min = 3))]
    pub forward_url: Option<String>,

    pub active: Option<bool>,

    #[validate(length(min = 1))]
    pub version: Option<String>,

    #[validate(length(min = 1))]
    pub role_namespaces: Option<Vec<String>>,

    #[validate(length(min = 1))]
    pub roles: Option<Vec<WebApiRole>>,

    #[validate(length(min = 1))]
    pub environment: Option<String>,
}

impl From<&WebRequestPartialApiService> for Vec<WebApiRole> {
    fn from(value: &WebRequestPartialApiService) -> Self {
        let mut all_roles = Self::new();
        if let Some(roles) = &value.roles {
            all_roles.extend(roles.clone());
        }
        if let Some(namespaces) = &value.role_namespaces {
            for namespace in namespaces {
                all_roles.push(WebApiRole {
                    id: None,
                    namespace: namespace.clone(),
                    name: NAMESPACE_MEMBER_ROLE.into(),
                })
            }
        }

        all_roles
    }
}

#[derive(Debug, Serialize, Deserialize, Validate, Clone, PartialEq, Eq)]
pub struct DbApiRole {
    pub id: Option<Thing>,
    #[validate(length(min = 3))]
    pub namespace: String,

    #[validate(length(min = 1))]
    pub name: String,
}

#[derive(Debug, Serialize, Deserialize, Validate, Clone)]
pub struct WebApiRole {
    pub id: Option<String>,

    #[validate(length(min = 3))]
    pub namespace: String,

    #[validate(length(min = 1))]
    pub name: String,
}

impl From<&WebApiRole> for DbApiRole {
    fn from(web_record: &WebApiRole) -> Self {
        Self {
            id: match &web_record.id {
                Some(web_id) => Some(Thing::from((API_ROLE_TABLE.to_string(), web_id.clone()))),
                _ => None,
            },
            namespace: web_record.namespace.clone(),
            name: web_record.name.clone(),
        }
    }
}

impl fmt::Display for DbApiRole {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "{}{}{}",
            self.namespace, ROLE_NAMESPACE_DELIMITER, self.name
        )
    }
}

impl From<&DbApiRole> for WebApiRole {
    fn from(db_record: &DbApiRole) -> Self {
        Self {
            id: match &db_record.id {
                Some(thing) => Some(format!("{}", thing.id)),
                _ => None,
            },
            namespace: db_record.namespace.clone(),
            name: db_record.name.clone(),
        }
    }
}

impl fmt::Display for WebApiRole {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "{}{}{}",
            self.namespace, ROLE_NAMESPACE_DELIMITER, self.name
        )
    }
}

#[derive(Debug, Serialize, Deserialize, Validate, Clone)]
pub struct RelatedAuthorizations {
    pub authorizations: Vec<Thing>,
}

#[derive(Debug, Serialize, Deserialize, Validate, Clone)]
pub struct RelatedMembers {
    pub members: Vec<Thing>,
}
