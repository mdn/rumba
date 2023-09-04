use std::{
    path::{Path, PathBuf},
    sync::Arc,
    time::Instant,
};

use anyhow::Error;
use async_openai::{
    config::OpenAIConfig,
    types::{ChatCompletionRequestMessage, ChatCompletionResponseMessage, Role::User},
};
use futures::{stream, StreamExt, TryStreamExt};
use rumba::{
    ai::ask::{prepare_ask_req, AskRequest},
    db,
    experiments::Experiments,
    settings::SETTINGS,
};
use serde::{Deserialize, Serialize};
use tokio::fs;

use crate::prompts;

#[derive(Serialize, Deserialize)]
pub struct Storage {
    pub req: AskRequest,
    pub res: Option<ChatCompletionResponseMessage>,
}

pub async fn ask_all(
    path: Option<impl AsRef<Path>>,
    out: impl AsRef<Path>,
    experiments: Option<Experiments>,
) -> Result<(), Error> {
    let out = &out;
    std::fs::create_dir_all(out)?;
    let supabase_pool = &{
        let uri = SETTINGS.db.supabase_uri.as_ref().expect("no supabase");
        db::establish_supa_connection(uri).await
    };

    let openai_client = &Arc::new(async_openai::Client::with_config(
        OpenAIConfig::new().with_api_key(&SETTINGS.ai.as_ref().expect("no ai settings").api_key),
    ));

    let prompts = prompts::read(path)?;
    let total_samples = prompts.len();
    let before = Instant::now();
    stream::iter(prompts.into_iter().enumerate())
        .map(Ok::<(usize, Vec<String>), Error>)
        .try_for_each_concurrent(10, |(i, prompts)| async move {
            println!("processing {:0>2}", i);
            let control_out = PathBuf::from(out.as_ref())
                .with_file_name(format!("{:3}", i))
                .with_extension("md");
            let messages = prompts
                .into_iter()
                .map(|prompt| ChatCompletionRequestMessage {
                    role: User,
                    content: Some(prompt),
                    name: None,
                    function_call: None,
                })
                .collect();
            if let Some(req) =
                prepare_ask_req(openai_client, supabase_pool, messages, experiments).await?
            {
                let mut res = openai_client.chat().create(req.req.clone()).await?;
                let res = res.choices.pop().map(|res| res.message);
                fs::write(
                    control_out,
                    serde_json::to_vec_pretty(&Storage { req, res })?,
                )
                .await?;
            }
            println!("finished {:0>2}", i);
            Ok(())
        })
        .await?;
    let after = Instant::now();
    println!(
        "Tested {} prompts in {} seconds",
        total_samples,
        after.duration_since(before).as_secs()
    );
    Ok(())
}
