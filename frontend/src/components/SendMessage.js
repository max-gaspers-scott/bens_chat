import { useState, useRef } from 'react';
import { api } from '../api/api';

function SendMessage({ chatId, senderId, onMessageSent }) {
  const [content, setContent] = useState('');
  const [error, setError] = useState('');
  const [loading, setLoading] = useState(false);
  const [imageFile, setImageFile] = useState(null);
  const [imagePreview, setImagePreview] = useState(null);
  const fileInputRef = useRef(null);

  const handleFileChange = (e) => {
    const file = e.target.files[0];
    if (!file) return;
    setImageFile(file);
    setImagePreview(URL.createObjectURL(file));
  };

  const clearImage = () => {
    setImageFile(null);
    setImagePreview(null);
    if (fileInputRef.current) fileInputRef.current.value = '';
  };

  const handleSubmit = async (e) => {
    e.preventDefault();
    if (!content.trim() && !imageFile) return;

    setError('');
    setLoading(true);

    try {
      let minioKey = null;

      if (imageFile) {
        const fileExtension = imageFile.name.split('.').pop();
        const { upload_url, object_key } = await api.getUploadUrl(chatId, fileExtension);
        await api.uploadFileToMinio(upload_url, imageFile);
        minioKey = object_key;
      }

      const result = await api.sendMessage({
        chat_id: chatId,
        content: content.trim() || '',
        minio_url: minioKey,
      });

      if (result.res === 'success') {
        setContent('');
        clearImage();
        onMessageSent();
      } else {
        setError(result.res || 'Failed to send message');
      }
    } catch (err) {
      setError(err.message || 'Failed to send message');
    } finally {
      setLoading(false);
    }
  };

  const canSend = !loading && (content.trim() || imageFile);

  return (
    <div className="send-message">
      {imagePreview && (
        <div className="image-preview">
          <img src={imagePreview} alt="Preview" />
          <button
            type="button"
            className="remove-image-btn"
            onClick={clearImage}
            title="Remove image"
          >
            ✕
          </button>
        </div>
      )}
      <form onSubmit={handleSubmit}>
        <input
          type="file"
          accept="image/*"
          ref={fileInputRef}
          onChange={handleFileChange}
          style={{ display: 'none' }}
        />
        <button
          type="button"
          className="attach-btn"
          onClick={() => fileInputRef.current?.click()}
          disabled={loading}
          title="Attach image"
        >
          📎
        </button>
        <input
          type="text"
          value={content}
          onChange={(e) => setContent(e.target.value)}
          placeholder="Type a message..."
          disabled={loading}
        />
        <button type="submit" disabled={!canSend}>
          {loading ? 'Sending...' : 'Send'}
        </button>
      </form>
      {error && <p className="error">{error}</p>}
    </div>
  );
}

export default SendMessage;
