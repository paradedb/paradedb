use opendal::services::S3;
use supabase_wrappers::prelude::*;
use thiserror::Error;

use super::options::*;

impl TryFrom<ServerOptions> for S3 {
    type Error = ObjectStoreError;

    fn try_from(options: ServerOptions) -> Result<Self, Self::Error> {
        let server_options = options.server_options();
        let user_mapping_options = options.user_mapping_options();

        let mut builder = S3::default();
        let bucket = require_option(AmazonServerOption::Bucket.as_str(), server_options)?;
        builder.bucket(bucket);

        if let Some(root) = server_options.get(AmazonServerOption::Root.as_str()) {
            builder.root(root);
        }

        if let Some(region) = server_options.get(AmazonServerOption::Region.as_str()) {
            builder.region(region);
        }

        if let Some(access_key_id) =
            user_mapping_options.get(AmazonUserMappingOption::AccessKeyId.as_str())
        {
            builder.access_key_id(access_key_id);
        }

        if let Some(secret_access_key) =
            user_mapping_options.get(AmazonUserMappingOption::SecretAccessKey.as_str())
        {
            builder.secret_access_key(secret_access_key);
        }

        if let Some(security_token) =
            user_mapping_options.get(AmazonUserMappingOption::SecurityToken.as_str())
        {
            builder.security_token(security_token);
        }

        if let Some(endpoint) = server_options.get(AmazonServerOption::Endpoint.as_str()) {
            builder.endpoint(endpoint);
        }

        if let Some(allow_anonymous) =
            server_options.get(AmazonServerOption::AllowAnonymous.as_str())
        {
            if allow_anonymous == "true" {
                builder.allow_anonymous();
            }
        }

        Ok(builder)
    }
}

#[derive(Error, Debug)]
pub enum ObjectStoreError {
    #[error(transparent)]
    OptionsError(#[from] OptionsError),

    #[error(transparent)]
    ObjectStore(#[from] object_store::Error),
}
