FROM python:3.12.11-slim
WORKDIR /app
COPY requirements.txt .
RUN pip install --no-cache-dir -r requirements.txt
COPY manager.py .
USER 10001:10001
CMD ["python", "-u", "manager.py"]
