import numpy as np
import hashlib

model = None
try:
    from sentence_transformers import SentenceTransformer
except ImportError:
    SentenceTransformer = None

def get_embedding(text):
    global model
    if SentenceTransformer is not None:
        if model is None:
            try:
                model = SentenceTransformer('all-MiniLM-L6-v2')
            except Exception as e:
                print(f"Error loading SentenceTransformer: {e}")
                model = "MOCK"
        
        if model != "MOCK":
            try:
                emb = model.encode(text)
                return emb.tolist()
            except Exception as e:
                print(f"Embedding encoding failed: {e}")
                
    # Determinstic 384-dimensional mock vector generator (fallback)
    hash_object = hashlib.md5(text.encode('utf-8'))
    seed = int(hash_object.hexdigest(), 16) % (2**32)
    rng = np.random.default_rng(seed)
    vector = rng.random(384)
    norm = np.linalg.norm(vector)
    if norm > 0:
        vector = vector / norm
    return vector.tolist()
