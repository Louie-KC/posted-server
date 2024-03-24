* `docker-compose up -d`

## MySql:
* `docker cp sql/schema.sql posted_mysql:/schema.sql`
* `docker cp sql/devtest_data.sql posted_mysql:/devtest_data.sql`

* `docker exec -it posted_mysql mysql -uroot -ppassword`

* `source schema.sql`
* `source devtest_data.sql`

## Redis:
* `docker exec -it redis_cache_posted redis-cli -a <password>`
