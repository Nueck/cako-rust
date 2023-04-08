use crate::entity::ArticleData;
use actix_web::web::Json;
use actix_web::{delete, get, post, put, web, HttpResponse, Responder};
use brave_config::app::AppState;
use brave_config::blog::generate_blog_table;
use brave_config::utils::jwt::UserDataInfo;
use brave_db::entity::article;
use brave_db::entity::article::Model;
use brave_db::entity::prelude::{Article, Users};
use sea_orm::ActiveValue::Set;
use sea_orm::{ActiveModelTrait, ColumnTrait, EntityTrait, QueryFilter, QueryOrder};
use serde::{Deserialize, Serialize};

//文章
pub fn article_config(cfg: &mut web::ServiceConfig) {
    cfg.service(get_articles_info)
        .service(get_article_data)
        .service(update_article_data)
        .service(save_article_data)
        .service(delete_article_data);
}

//获取文章列表
#[get("/articles")]
async fn get_articles_info(
    data: web::Data<AppState>,
    token: web::ReqData<UserDataInfo>,
) -> impl Responder {
    let db = &data.conn;
    let id = &token.id;

    //获取数据库中文章信息
    match Article::find()
        .filter(article::Column::UserId.eq(id.clone().to_owned()))
        .order_by_desc(article::Column::ArticleId.to_owned())
        .all(db)
        .await
    {
        Ok(table) => {
            #[derive(Clone, Deserialize, Serialize)]
            struct ArticleData {
                table_id: i64,
                title: String,
                img_url: String,
            }
            let data = table
                .into_iter()
                // .flat_map(|models: Vec<Model>| {
                //     models
                //         .into_iter()
                .map(|model: Model| ArticleData {
                    table_id: model.article_id,
                    title: model.title.unwrap(),
                    img_url: model.img_url.unwrap(),
                })
                .collect::<Vec<ArticleData>>()
                .to_vec();
            // })
            // .collect::<Vec<ArticleData>>();
            HttpResponse::Ok().json(serde_json::json!({"state": "success", "data": data }))
        }
        Err(_) => {
            const MSG: &str = "Unable to find the data";
            HttpResponse::Ok().json(serde_json::json!({"state": "error", "message": MSG }))
        }
    }
}

//获取文章编辑信息
#[get("/article/{id}")]
async fn get_article_data(
    data: web::Data<AppState>,
    token: web::ReqData<UserDataInfo>,
    path: web::Path<i64>,
) -> impl Responder {
    let db = &data.conn;
    let id = &token.id;
    let table_id = path.into_inner();

    //获取数据库中文章信息
    match Article::find_by_id(table_id)
        .filter(article::Column::UserId.eq(id.clone().to_owned()))
        .one(db)
        .await
        .expect("Unable to find the article table")
    {
        None => {
            const MSG: &str = "Unable to find the data";
            HttpResponse::Ok().json(serde_json::json!({"state": "error", "message": MSG }))
        }
        Some(model) => {
            #[derive(Clone, Deserialize, Serialize)]
            struct ArticleEditData {
                title: String,
                subtitle: String,
                img_url: String,
                content: String,
            }

            let data = ArticleEditData {
                title: model.title.unwrap(),
                subtitle: model.subtitle.unwrap(),
                img_url: model.img_url.unwrap(),
                content: model.content.unwrap(),
            };

            HttpResponse::Ok().json(serde_json::json!({"state": "success", "data": data }))
        }
    }
}

#[post("/article")]
async fn save_article_data(
    data: web::Data<AppState>,
    token: web::ReqData<UserDataInfo>,
    json: Json<ArticleData>,
) -> impl Responder {
    let db = &data.conn;
    let id = &token.id;
    let name = &token.aud;

    match Users::find_by_id(id.clone().to_owned())
        .one(db)
        .await
        .expect("Could not find Article -- saveArticleEditData")
    {
        None => {
            const MSG: &str = "User not find";
            HttpResponse::Ok().json(serde_json::json!({"state": "error", "message": MSG }))
        }
        Some(user) => {
            let model = article::ActiveModel {
                user_id: Set(user.user_id.to_owned()),
                title: Set(Some(json.title.to_owned())),
                content: Set(Some(json.content.to_owned())),
                img_url: Set(Some(json.img_url.to_owned())),
                html_content: Set(Some(json.html_content.to_owned())),
                subtitle: Set(Some(json.subtitle.to_owned())),
                ..Default::default()
            };

            //更新数据
            match model.insert(db).await {
                Ok(table) => {
                    let mut model: article::ActiveModel = table.into();
                    let id = model.article_id.clone().unwrap();
                    model.url = Set(Some(generate_blog_table(&name, &id)));

                    match model.update(db).await {
                        Ok(_) => {
                            const MSG: &str = "Save data successfully";
                            HttpResponse::Ok()
                                .json(serde_json::json!({"state": "success", "message": MSG }))
                        }
                        Err(_) => {
                            const MSG: &str = "Failed to Save data url";
                            HttpResponse::Ok()
                                .json(serde_json::json!({"state": "error", "message": MSG }))
                        }
                    }
                }
                Err(_) => {
                    const MSG: &str = "Failed to Save data";
                    HttpResponse::Ok().json(serde_json::json!({"state": "error", "message": MSG }))
                }
            }
        }
    }
}

///用于文章更新
#[put("/article")]
async fn update_article_data(
    data: web::Data<AppState>,
    token: web::ReqData<UserDataInfo>,
    json: Json<ArticleData>,
) -> impl Responder {
    let db = &data.conn;
    let id = &token.id;

    match Article::find_by_id(json.table_id.clone().to_owned())
        .filter(article::Column::UserId.eq(id.clone().to_owned()))
        .one(db)
        .await
        .expect("Could not find Article -- updateArticleEditData")
    {
        None => {
            const MSG: &str = "Unable to find the data";
            HttpResponse::Ok().json(serde_json::json!({"state": "error", "message": MSG }))
        }
        Some(table) => {
            let mut model: article::ActiveModel = table.into();
            model.html_content = Set(Some(json.html_content.to_owned()));
            model.title = Set(Some(json.title.to_owned()));
            model.subtitle = Set(Some(json.subtitle.to_owned()));
            model.content = Set(Some(json.content.to_owned()));
            model.img_url = Set(Some(json.img_url.to_owned()));

            //更新数据
            match model.update(db).await {
                Ok(_) => {
                    const MSG: &str = "Update data successfully";
                    HttpResponse::Ok()
                        .json(serde_json::json!({"state": "success", "message": MSG }))
                }
                Err(_) => {
                    const MSG: &str = "Failed to update data";
                    HttpResponse::Ok().json(serde_json::json!({"state": "error", "message": MSG }))
                }
            }
        }
    }
}

//删除文章
#[delete("/article/{id}")]
async fn delete_article_data(
    data: web::Data<AppState>,
    token: web::ReqData<UserDataInfo>,
    path: web::Path<i64>,
) -> impl Responder {
    let db = &data.conn;
    let id = &token.id;
    let table_id = path.into_inner();

    match Article::find_by_id(table_id)
        .filter(article::Column::UserId.eq(id.clone().to_owned()))
        .one(db)
        .await
        .expect("Could not find Article -- deleteArticleEditData")
    {
        None => {
            const MSG: &str = "Unable to find the data";
            HttpResponse::Ok().json(serde_json::json!({"state": "error", "message": MSG }))
        }
        Some(table) => {
            let model: article::ActiveModel = table.into();

            //删除数据
            match model.delete(db).await {
                Ok(_) => {
                    const MSG: &str = "Delete data successfully";
                    HttpResponse::Ok()
                        .json(serde_json::json!({"state": "success", "message": MSG }))
                }
                Err(_) => {
                    const MSG: &str = "Failed to Delete data";
                    HttpResponse::Ok().json(serde_json::json!({"state": "error", "message": MSG }))
                }
            }
        }
    }
}
