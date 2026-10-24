docker compose down
docker compose up -d postgres

for i in $(seq 1 5);
do
    echo "Polling postgres-music db..."
    status=$(docker inspect --format='{{json .State.Health}}' postgres-music)
    if [[ $status == *"healthy"* ]];
    then
        break
    fi
    sleep 5
done
echo "postgres-music is healthy"

cargo sqlx migrate run
rm -rf .sqlx
cargo sqlx prepare -- --all-targets --all-features