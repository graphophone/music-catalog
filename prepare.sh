docker compose down
docker compose up -d postgres

for i in $(seq 1 5);
do
    echo "Polling postgres-tracks db..."
    status=$(docker inspect --format='{{json .State.Health}}' postgres-tracks)
    if [[ $status == *"healthy"* ]];
    then
        break
    fi
    sleep 5
done
echo "postgres-tracks is healthy"

cargo sqlx migrate run
rm -rf .sqlx
cargo sqlx prepare -- --all-targets --all-features