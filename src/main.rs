use actix_multipart::{Field, Multipart};
use actix_web::{web, App, HttpRequest, HttpResponse, HttpServer, Responder};
use async_zip::{write::*, Compression};
use futures::StreamExt;
use tokio::io::AsyncWriteExt;
use uuid::Uuid;

async fn multipart(multipart: Multipart) -> impl Responder {
    async_create_archive(multipart).await;
    HttpResponse::Ok().finish()
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    HttpServer::new(|| App::new().route("/multipart", web::post().to(multipart)))
        .bind(("127.0.0.1", 8000))?
        .run()
        .await
}

pub async fn async_create_archive(mut body: Multipart) -> Result<String, anyhow::Error> {
    let archive_name = format!("tmp/{}", Uuid::new_v4());
    let mut archive = tokio::fs::File::create(archive_name.clone()).await.unwrap();
    let mut writer = ZipFileWriter::new(&mut archive);

    while let Some(item) = body.next().await {
        let mut field = item.unwrap();

        let mut uncompressed_size = 0;
        let content_disposition = field.content_disposition();

        let filename = content_disposition
            .get_filename()
            .map_or_else(|| Uuid::new_v4().to_string(), sanitize_filename::sanitize);

        let opts = EntryOptions::new(filename.clone(), Compression::Stored);

        let mut entry_writer = writer.write_entry_stream(opts).await.unwrap();
        while let Some(chunk) = field.next().await {
            let mut chunk_ptr = chunk.unwrap();
            uncompressed_size += chunk_ptr.len();
            entry_writer.write_all_buf(&mut chunk_ptr).await.unwrap();
        }
        print!(
            "FILE NAME: {:?}, UNCOMPRESSED SIZE: {:?}",
            filename.clone(),
            uncompressed_size
        );
        entry_writer.close().await.unwrap();
    }
    writer.close().await.unwrap();
    archive.shutdown().await.unwrap();

    Ok(archive_name)
}
