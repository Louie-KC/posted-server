* `docker-compose up -d`

* `docker cp sql/schema.sql posted_mysql:/schema.sql`
* `docker cp sql/devtest_data.sql posted_mysql:/devtest_data.sql`

* `docker exec -it posted_mysql mysql -uroot -ppassword`

* `source schema.sql`
* `source devtest_data.sql`